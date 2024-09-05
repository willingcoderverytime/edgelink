import asyncio
import json
import os
import platform
import subprocess
import signal
import copy


async def start_edgelink_process(el_args: list[str]):
    # Determine the operating system and choose the appropriate executable name
    if platform.system() == 'Windows':
        createion_flags = subprocess.CREATE_NEW_PROCESS_GROUP
        myprog_name = 'edgelinkd.exe'
    else:
        createion_flags = 0
        myprog_name = 'edgelinkd'

    # Get the path of the current script
    script_dir = os.path.dirname(os.path.abspath(__file__))

    target = os.getenv('EDGELINK_BUILD_TARGET', '')
    profile = os.getenv('EDGELINK_BUILD_PROFILE', 'release')

    # Construct the full path to the myprog executable
    myprog_path = os.path.join(
        script_dir, '..', 'target', target, profile, myprog_name)

    # Start the process
    process = await asyncio.create_subprocess_exec(
        myprog_path, *el_args,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
        stdin=asyncio.subprocess.PIPE,
        creationflags=createion_flags
    )
    return process


async def read_json_from_process(process, nexpected: int):
    # Read from the process's stdout
    buffer = ''
    counter = 0
    try:
        while True:
            line = await asyncio.wait_for(process.stdout.readline(), timeout=8)
            if not line:
                break
            buffer += line.decode('utf-8')

            # Look for delimiters \x1E and \n
            while '\x1E' in buffer:
                start, rest = buffer.split('\x1E', 1)
                if '\n' in rest:
                    json_str, buffer = rest.split('\n', 1)
                    try:
                        json_obj = json.loads(json_str)
                        counter += 1
                        yield json_obj
                        if counter >= nexpected:
                            if platform.system() == 'Windows':
                                # Send CTRL+C signal
                                process.send_signal(signal.CTRL_BREAK_EVENT)
                            else:
                                process.send_signal(signal.SIGINT)
                            # Wait for the process to respond and exit
                            await process.wait()  # Wait for the process to finish
                            return
                    except json.JSONDecodeError as e:
                        print(f"JSON decode error: {e}")
                        raise e
                else:
                    break
    except asyncio.TimeoutError as e:
        print("Timeout: No more output in 8 seconds")
        raise e
        # await asyncio.sleep(2)  # Wait for the process to respond and exit
    # Ensure the process exits completely
    await process.wait()


async def run_edgelink_with_stdin(input_data: bytes, nexpected: int) -> list[dict]:
    el_args = ['-v', '0', '--stdin']
    msgs = []
    process = await start_edgelink_process(el_args)
    try:
        process.stdin.write(input_data)
        process.stdin.close()
        async for i in read_json_from_process(process, nexpected):
            msgs.append(i)
        return msgs
    except Exception as e:
        print(e)
        process.kill()
        await process.wait()
        raise e


async def run_edgelink(flows_path: str, nexpected: int) -> list[dict]:
    el_args = ['-v', '0', '-f', flows_path]
    msgs = []
    process = await start_edgelink_process(el_args)
    try:
        async for i in read_json_from_process(process, nexpected):
            msgs.append(i)
        return msgs
    except Exception as e:
        print(e)
        process.kill()
        await process.wait()
        raise e


async def run_with_single_node_ntimes(payload_type: str | None, payload, node_json: object,
                                      nexpected: int, once: bool = False, topic: str | None = None):
    inject = {
        "id": "1",
        "type": "inject",
        "z": "0",
        "name": "",
        "props": [],  # [{"p": "payload"}, {"p": "topic", "vt": "str"}],
        "repeat": once and '' or '0',
        "crontab": "",
        "once": once,
        "onceDelay": 0,
        "topic": topic,
        "wires": [["2"]]
    }
    if payload != None:
        inject['props'].append({'p': 'payload'})
        inject["payload"] = str(payload)
        inject["payloadType"] = payload_type
    if topic != None:
        inject['props'].append({'p': 'topic', 'vt': 'str'})
    user_node = copy.deepcopy(node_json)
    user_node["id"] = "2"
    user_node["z"] = "0"
    if 'wires' not in node_json:
        user_node["wires"] = [["3"]]
    console_node = {"id": "3", "type": "console-json", "z": "0"}
    final_flows_json = [{"id": "0", "type": "tab"},
                        inject, user_node, console_node]
    flows_text = json.dumps(final_flows_json, ensure_ascii=False)
    print("INPUT_JSON:\n", flows_text)
    flow_bytes = flows_text.encode('utf-8')

    return await run_edgelink_with_stdin(flow_bytes, nexpected)


async def run_flow_with_msgs_ntimes(flows_obj: list[object],
                                    msgs: list[object] | None,
                                    nexpected: int, injectee_node_id: str = '1'):
    flow_bytes = json.dumps(flows_obj, ensure_ascii=False).encode('utf-8')

    input_bytes = bytearray()
    input_bytes.append(0x1E)
    input_bytes.extend(flow_bytes)
    input_bytes.append(0x0A)  # \n
    for msg in msgs:
        msg_injection = None
        if 'nid' in msg and 'msg' in msg: # We got a raw injection
            msg_injection = msg
        else:
            msg_injection = {'nid': injectee_node_id, 'msg': msg}
        inj_bytes = json.dumps(
            msg_injection, ensure_ascii=False).encode('utf-8')
        input_bytes.append(0x1E)
        input_bytes.extend(inj_bytes)
        input_bytes.append(0x0A)  # \n

    print("INPUT_JSON_SEQ:\n", input_bytes)
    #with open("c:\\tmp\\hello.dat", "wb") as f:
    #    f.write(input_bytes)
    return await run_edgelink_with_stdin(bytes(input_bytes), nexpected)


async def run_single_node_with_msgs_ntimes(node_json: object, msgs: list[object] | None,
                                           nexpected: int, injectee_node_id: str = '1'):
    user_node = copy.deepcopy(node_json)
    user_node["id"] = "1"
    user_node["z"] = "0"
    if 'wires' not in node_json:
        user_node["wires"] = [["2"]]
    console_node = {"id": "2", "type": "console-json", "z": "0"}
    final_flows_json = [{"id": "0", "type": "tab"}, user_node, console_node]
    return await run_flow_with_msgs_ntimes(final_flows_json, msgs, nexpected, injectee_node_id)

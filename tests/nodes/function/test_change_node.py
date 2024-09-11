import json
import pytest
import time

from tests import *


# 0001 should load node with defaults
# 0002 should load defaults if set to replace
# 0003 should load defaults if set to change
# 0004 should no-op if there are no rules

@pytest.mark.describe('change Node')
class TestChangeNode:

    @pytest.mark.describe('#set')
    class TestSet:

        @pytest.mark.asyncio
        @pytest.mark.it('''sets the value of the message property''')
        async def test_set_1(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100", "action": "replace", "property": "payload",
                    "from": "", "to": "changed", "reg": False, "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg":  {'payload': 'changeMe'}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert msgs[0]["payload"] == 'changed'

        # sets the value of global context property
        # sets the value of persistable global context property

        @pytest.mark.asyncio
        @pytest.mark.it('''sets the value and type of the message property''')
        async def test_set_4(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100", "rules": [
                    {"t": "set", "p": "payload", "pt": "msg",
                        "to": "12345", "tot": "num"}
                ],
                    "reg": False, "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg":  {'payload': 'changeMe'}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            payload = msgs[0]['payload']
            assert isinstance(payload, float) or isinstance(payload, int)
            assert payload == 12345

        @pytest.mark.asyncio
        @pytest.mark.it('''sets the value of an already set multi-level message property''')
        async def test_set_5(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100", "action": "replace", "property": "foo.bar",
                 "from": "", "to": "bar", "reg": False, "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg":  {"foo": {"bar": "foo"}}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert msgs[0]['foo']['bar'] == "bar"

        @pytest.mark.asyncio
        @pytest.mark.it('''sets the value of an empty multi-level message property''')
        async def test_set_6(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100", "action": "replace", "property": "foo.bar",
                 "from": "", "to": "bar", "reg": False, "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg":  {}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert msgs[0]['foo']['bar'] == "bar"

        @pytest.mark.asyncio
        @pytest.mark.it('''sets the value of a message property to another message property''')
        async def test_set_7(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100", "action": "replace", "property": "foo",
                 "from": "", "to": "msg.fred", "reg": False, "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg": {"fred": "bar"}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert msgs[0]['foo'] == "bar"

        @pytest.mark.asyncio
        @pytest.mark.it('''sets the value of a multi-level message property to another multi-level message property''')
        async def test_set_8(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100", "action": "replace", "property": "foo.bar",
                 "from": "", "to": "msg.fred.red", "reg": False, "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg": {"fred": {"red": "bar"}}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert msgs[0]['foo']['bar'] == "bar"

        @pytest.mark.asyncio
        @pytest.mark.it('''doesn't set the value of a message property when the 'to' message property does not exist''')
        async def test_set_9(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100", "action": "replace", "property": "foo.bar",
                 "from": "", "to": "msg.fred.red", "reg": False, "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg": {}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert 'foo' not in msgs[0]

        @pytest.mark.asyncio
        @pytest.mark.it('''overrides the value of a message property when the 'to' message property does not exist''')
        async def test_set_10(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100", "action": "replace", "property": "payload",
                 "from": "", "to": "msg.foo", "reg": False, "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg": {"payload": "Hello"}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert 'payload' not in msgs[0]

        @pytest.mark.asyncio
        @pytest.mark.it('''sets the message property to null when the 'to' message property equals null''')
        async def test_set_11(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100", "action": "replace", "property": "payload",
                 "from": "", "to": "msg.foo", "reg": False, "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg": {"payload": "Hello", "foo": None}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert 'payload' in msgs[0]
            assert msgs[0]['payload'] == None

        @pytest.mark.asyncio
        @pytest.mark.it('''does not set other properties using = inside to property''')
        async def test_set_12(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100", "action": "replace", "property": "payload",
                 "from": "", "to": "msg.otherProp=10", "reg": False, "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg": {"payload": "changeMe"}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert 'payload' not in msgs[0]

        @pytest.mark.asyncio
        @pytest.mark.it('''splits dot delimited properties into objects''')
        async def test_set_13(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100", "action": "replace", "property": "pay.load",
                 "from": "", "to": "10", "reg": False, "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg": {"pay": {"load": "changeMe"}}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert msgs[0]['pay']['load'] == "10"

        # changes the value to flow context property
        # changes the value to persistable flow context property
        # changes the value to global context property
        # changes the value to persistable global context property

        @pytest.mark.asyncio
        @pytest.mark.it('''changes the value to a number''')
        async def test_set_18(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100",
                 "rules": [{"t": "set", "p": "payload", "to": "123", "tot": "num"}], "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg": {"payload": ""}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert msgs[0]['payload'] == 123

        @pytest.mark.asyncio
        @pytest.mark.it('''changes the value to a boolean value''')
        async def test_set_19(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100",
                 "rules": [{"t": "set", "p": "payload", "to": "true", "tot": "bool"}], "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg": {"payload": ""}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert msgs[0]['payload'] == True

        @pytest.mark.asyncio
        @pytest.mark.it('''changes the value to a js object''')
        async def test_set_20(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100",
                 "rules": [{"t": "set", "p": "payload", "to": '{"a":123}', "tot": "json"}], "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg": {"payload": ""}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert msgs[0]['payload'] == {"a": 123}

        @pytest.mark.asyncio
        @pytest.mark.it('''changes the value to a buffer object''')
        async def test_set_21(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100",
                 "rules": [{"t": "set", "p": "payload", "to": '[72,101,108,108,111,32,87,111,114,108,100]', "tot": "bin"}], "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg": {"payload": ""}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert msgs[0]['payload'] == [72, 101, 108,
                                          108, 111, 32, 87, 111, 114, 108, 100]

        @pytest.mark.asyncio
        @pytest.mark.it('''sets the value of the message property to the current timestamp''')
        async def test_set_22(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100",
                 "rules": [{"t": "set", "p": "ts", "pt": "msg", "to": "", "tot": "date"}], "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg": {"payload": time.time_ns() / 1000_000.0}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert ((time.time_ns() / 1000_000.0) - msgs[0]['ts']) < 50000.0

        @pytest.mark.describe('env var')
        class TestSetEnvVar:

            def setup_method(self, method):
                os.environ["NR_TEST_A"] = "foo"

            def teardown_method(self, method):
                del os.environ["NR_TEST_A"]

            @pytest.mark.asyncio
            @pytest.mark.it('sets the value using env property')
            async def test_set_env_1(self):
                flows = [
                    {"id": "100", "type": "tab"},  # flow 1
                    {"id": "1", "type": "change", "z": "100",
                     "rules": [{"t": "set", "p": "payload", "pt": "msg", "to": "NR_TEST_A", "tot": "env"}], "name": "changeNode", "wires": [["2"]]},
                    {"id": "2", "z": "100", "type": "console-json"}
                ]
                injections = [
                    {"nid": "1", "msg": {"payload": "123", "topic": "ABC"}},
                ]
                msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
                assert msgs[0]["payload"] == "foo"

            @pytest.mark.asyncio
            @pytest.mark.it('sets the value using env property from tab')
            async def test_set_env_2(self):
                flows = [
                    {"id": "100", "type": "tab", "env": [
                        {"name": "NR_TEST_A", "value": "bar", "type": "str"}
                    ]},  # flow 1
                    {"id": "1", "type": "change", "z": "100",
                     "rules": [{"t": "set", "p": "payload", "pt": "msg", "to": "NR_TEST_A", "tot": "env"}], "name": "changeNode", "wires": [["2"]]},
                    {"id": "2", "z": "100", "type": "console-json"}
                ]
                injections = [
                    {"nid": "1", "msg": {"payload": "123", "topic": "ABC"}},
                ]
                msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
                assert msgs[0]["payload"] == "bar"

            @pytest.mark.asyncio
            @pytest.mark.it('sets the value using env property from group')
            async def test_set_env_3(self):
                flows = [
                    {"id": "100", "type": "tab"},  # flow 1
                    {"id": "999", "type": "group", "env": [
                        {"name": "NR_TEST_A",
                         "value": "bar", "type": "str"}
                    ], "z": "100"},
                    {"id": "1", "type": "change", "z": "100", "g": "999",
                     "rules": [{"t": "set", "p": "payload", "pt": "msg", "to": "NR_TEST_A", "tot": "env"}], "name": "changeNode", "wires": [["2"]]},
                    {"id": "2", "z": "100", "type": "console-json"}
                ]
                injections = [
                    {"nid": "1", "msg": {"payload": "123", "topic": "ABC"}},
                ]
                msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
                assert msgs[0]["payload"] == "bar"

            @pytest.mark.asyncio
            @pytest.mark.it('sets the value using env property from nested group')
            async def test_set_env_4(self):
                flows = [
                    {"id": "100", "type": "tab"},  # flow 1
                    {"id": "999", "type": "group", "env": [
                        {"name": "NR_TEST_A",
                         "value": "bar", "type": "str"}
                    ], "z": "100"},
                    {"id": "998", "type": "group",
                        "g": "999", "env": [], "z": "100"},
                    {"id": "1", "type": "change", "z": "100", "g": "998",
                     "rules": [{"t": "set", "p": "payload", "pt": "msg", "to": "NR_TEST_A", "tot": "env"}], "name": "changeNode", "wires": [["2"]]},
                    {"id": "2", "z": "100", "type": "console-json"}
                ]
                injections = [
                    {"nid": "1", "msg": {"payload": "123", "topic": "ABC"}},
                ]
                msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
                assert msgs[0]["payload"] == "bar"

        @pytest.mark.skip #FIXME
        @pytest.mark.asyncio
        @pytest.mark.it('''sets the value of a message property using a nested property''')
        async def test_set_28(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "name": "", "z": "100", "rules": [
                    {"t": "set", "p": "payload", "pt": "msg", "to": "lookup[msg.topic]", "tot": "msg"}],
                    "action": "", "property": "", "from": "", "to": "", "reg": False, "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {
                    "nid": "1",
                    "msg": {"payload": "", "lookup": {"a": 1, "b": 2}, "topic": "b"}
                },
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert msgs[0]["payload"] == 2

        @pytest.mark.skip #FIXME
        @pytest.mark.asyncio
        @pytest.mark.it('''sets the value of a nested message property using a message property''')
        async def test_set_29(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "name": "", "Z": "100", "rules": [
                    {"t": "set", "p": "lookup[msg.topic]",
                        "pt": "msg", "to": "payload", "tot": "msg"}],
                 "action": "", "property": "", "from": "", "to": "", "reg": False, "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {
                    "nid": "1",
                    "msg": {"payload": "newValue", "lookup": {"a": 1, "b": 2}, "topic": "b"}
                },
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert msgs[0]["lookup"]["b"] == "newValue"


# 23 changes the value using jsonata
# 24 reports invalid jsonata expression
# 25 changes the value using flow context with jsonata
# 26 changes the value using persistable flow context with jsonata
# 27 changes the value using persistable global context with jsonata
# 28 sets the value of a message property using a nested property
# 0037 sets the value of a nested message property using a message property
# 0038 sets the value of a message property using a nested property in flow context
# 0039 sets the value of a message property using a nested property in flow context
# 0040 sets the value of a nested flow context property using a message property
# 0041 deep copies the property if selected

    @pytest.mark.describe('#change')
    class TestChange:

        @pytest.mark.asyncio
        @pytest.mark.it('changes the value of the message property')
        async def test_0042(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100", "action": "change", "property": "payload", "from": "Hello",
                    "to": "Goodbye", "reg": False, "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg": {"payload": "Hello World!"}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert msgs[0]["payload"] == "Goodbye World!"

        @pytest.mark.asyncio
        @pytest.mark.it('''changes the value and doesnt change type of the message property for partial match''')
        async def test_0043(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100", "rules": [
                    {"t": "change", "p": "payload", "pt": "msg", "from": "123",
                     "fromt": "str", "to": "456", "tot": "num"}], "reg": False, "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg": {"payload": "Change123Me"}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert msgs[0]["payload"] == "Change456Me"

        @pytest.mark.asyncio
        @pytest.mark.it('''changes the value and type of the message property if a complete match - number''')
        async def test_0044(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100",
                 "rules": [
                     {"t": "change", "p": "payload", "pt": "msg", "from": "123", "fromt": "str", "to": "456", "tot": "num"}],
                 "reg": False, "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg": {"payload": "123"}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert msgs[0]["payload"] == 456

        @pytest.mark.asyncio
        @pytest.mark.it('''changes the value and type of the message property if a complete match - boolean''')
        async def test_0045(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100",
                 "rules": [
                     {"t": "change", "p": "payload.a", "pt": "msg", "from": "123",
                      "fromt": "str", "to": "true", "tot": "bool"},
                     {"t": "change", "p": "payload.b", "pt": "msg", "from": "456",
                      "fromt": "str", "to": "false", "tot": "bool"}
                 ],
                    "reg": False, "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg": {"payload": {"a": "123", "b": "456"}}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert msgs[0]["payload"]["a"] == True
            assert msgs[0]["payload"]["b"] == False

        @pytest.mark.asyncio
        @pytest.mark.it('''changes the value of a multi-level message property''')
        async def test_0046(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "action": "change", "z": "100",
                 "property": "foo.bar", "from": "Hello",
                    "to": "Goodbye", "reg": False, "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg": {"foo": {"bar": "Hello World!"}}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert msgs[0]["foo"]["bar"] == "Goodbye World!"

        @pytest.mark.asyncio
        @pytest.mark.it('''sends unaltered message if the changed message property does not exist''')
        async def test_0047(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100", "action": "change", "property": "foo", "from": "Hello",
                    "to": "Goodbye", "reg": False, "name": "changeNode", "wires": [["2"]]},
                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg": {"payload": "Hello World!"}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert msgs[0]["payload"] == "Hello World!"

        @pytest.mark.asyncio
        @pytest.mark.it('''sends unaltered message if a changed multi-level message property does not exist''')
        async def test_0048(self):
            flows = [
                {"id": "100", "type": "tab"},  # flow 1
                {"id": "1", "type": "change", "z": "100", "action": "change", "property": "foo.bar", "from": "Hello",
                    "to": "Goodbye", "reg": False, "name": "changeNode", "wires": [["2"]]},

                {"id": "2", "z": "100", "type": "console-json"}
            ]
            injections = [
                {"nid": "1", "msg": {"payload": "Hello World!"}},
            ]
            msgs = await run_flow_with_msgs_ntimes(flows, injections, 1)
            assert msgs[0]["payload"] == "Hello World!"

# 0049 changes the value of the message property based on a regex
# 0050 supports regex groups
# 0051 reports invalid regex
# 0052 supports regex groups - new rule format
# 0053 changes the value using msg property
# 0054 changes the value using flow context property
# 0055 changes the value using persistable flow context property
# 0056 changes the value using global context property
# 0057 changes the value using persistable global context property
# 0058 changes the number using global context property
# 0059 changes the number using persistable global context property
# 0060 changes the value using number - string payload
# 0061 changes the value using number - number payload
# 0062 changes the value using boolean - string payload
# 0063 changes the value using boolean - boolean payload
# 0064 changes the value of the global context

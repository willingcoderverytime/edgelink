import pytest
import os

from tests import *

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

# 0001 should do something with the catch node

@pytest.mark.asyncio
async def test_0002():
    '''should send returned message()'''
    node = {
        "type": "function",
        "func": "return msg;",
    }
    msgs = await run_with_single_node_ntimes('str', 'foo', node, 1, once=True, topic='bar')
    assert msgs[0]['topic'] == 'bar'
    assert msgs[0]['payload'] == 'foo'




# 0004 should send returned message using send()
# 0005 should allow accessing node.id and node.name and node.outputCount

# 0006 should clone single message sent using send()
# 0007 should not clone single message sent using send(,false)

# 0008 should clone first message sent using send() - array 1
# 0009 should clone first message sent using send() - array 2
# 0010 should clone first message sent using send() - array 3
# 0011 should clone first message sent using send() - array 3
# 0012 should pass through _topic

# TODO FIXME
@pytest.mark.asyncio
async def test_00013():
    '''should send to multiple outputs'''
    node = {
        "type": "function",
        "func": "var msg2 = RED.util.cloneMessage(msg); msg2.payload='p2'; return [msg, msg2];",
        "wires": [["3"], ["3"]]
    }
    msgs = await run_with_single_node_ntimes('str', 'foo', node, 2, once=True, topic='bar')
    assert msgs[0]['topic'] == 'bar'
    assert msgs[0]['topic'] == msgs[1]['topic']
    assert msgs[0]['payload'] != msgs[1]['payload']
    assert sorted([msgs[0]['payload'], msgs[1]['payload']]) == ['foo', 'p2']

# 0014 should send to multiple messages
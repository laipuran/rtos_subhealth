import pytest

from mock_exec_layer.contract import (
    InvalidPayload,
    parse_payload,
    UnsupportedPrimitive,
)


def test_hold_payload_is_empty_object():
    assert parse_payload('hold', '{}') == {}


@pytest.mark.parametrize('payload', ['[]', '{"unexpected":1}', 'not-json'])
def test_hold_rejects_invalid_payload(payload):
    with pytest.raises(InvalidPayload):
        parse_payload('hold', payload)


def test_go_to_tag_payload_contains_one_signed_32_bit_integer():
    assert parse_payload('go_to_tag', '{"target_tag":42}') == {'target_tag': 42}
    assert parse_payload('go_to_tag', '{"target_tag":-1}') == {'target_tag': -1}


@pytest.mark.parametrize(
    'payload',
    [
        '{}',
        '{"target_tag":true}',
        '{"target_tag":2147483648}',
        '{"target_tag":-2147483649}',
        '{"target_tag":1,"extra":2}',
    ],
)
def test_go_to_tag_rejects_invalid_payload(payload):
    with pytest.raises(InvalidPayload):
        parse_payload('go_to_tag', payload)


def test_unknown_primitive_is_rejected():
    with pytest.raises(UnsupportedPrimitive):
        parse_payload('dance', '{}')

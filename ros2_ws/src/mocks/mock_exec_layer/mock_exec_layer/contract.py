"""Primitive payload validation for the mock execution layer."""

import json


class UnsupportedPrimitive(ValueError):
    """Raised when a primitive is not supported by the mock."""


class InvalidPayload(ValueError):
    """Raised when a primitive payload violates its schema."""


def parse_payload(primitive: str, payload_json: str) -> dict:
    """Parse and validate the JSON payload for a supported primitive."""
    try:
        payload = json.loads(payload_json)
    except (TypeError, json.JSONDecodeError) as error:
        raise InvalidPayload('payload_json must contain valid JSON') from error

    if not isinstance(payload, dict):
        raise InvalidPayload('payload must be a JSON object')

    if primitive == 'hold':
        if payload:
            raise InvalidPayload('hold payload must be empty')
        return payload

    if primitive == 'go_to_tag':
        if set(payload) != {'target_tag'}:
            raise InvalidPayload('go_to_tag payload requires only target_tag')
        target_tag = payload['target_tag']
        if isinstance(target_tag, bool) or not isinstance(target_tag, int):
            raise InvalidPayload('target_tag must be an integer')
        if target_tag < -(2**31) or target_tag > 2**31 - 1:
            raise InvalidPayload('target_tag must fit in a signed 32-bit integer')
        return payload

    raise UnsupportedPrimitive(primitive)

"""Pure deterministic execution behavior for supported primitives."""

from dataclasses import dataclass

from .contract import UnsupportedPrimitive


@dataclass(frozen=True)
class ExecutionStep:
    progress: float
    phase: str
    details: dict


def execution_steps(primitive: str, payload: dict) -> tuple[ExecutionStep, ...]:
    """Build deterministic feedback steps for one validated task payload."""
    if primitive == 'hold':
        return (ExecutionStep(progress=1.0, phase='holding', details={}),)

    if primitive == 'go_to_tag':
        target_tag = payload['target_tag']
        return tuple(
            ExecutionStep(
                progress=index / 3,
                phase='moving_to_tag',
                details={
                    'current_tag': target_tag if index == 3 else -1,
                    'next_tag': -1 if index == 3 else target_tag,
                },
            )
            for index in range(1, 4)
        )

    raise UnsupportedPrimitive(primitive)


def canceled_feedback(last_step: ExecutionStep | None) -> ExecutionStep:
    """Build terminal cancellation feedback from the last completed step."""
    details = dict(last_step.details) if last_step is not None else {}
    if 'next_tag' in details:
        details['next_tag'] = -1
    return ExecutionStep(
        progress=last_step.progress if last_step is not None else 0.0,
        phase='canceled',
        details=details,
    )


def validate_step_delay(value: float) -> float:
    """Validate and return the configured deterministic step delay."""
    if value < 0.0:
        raise ValueError(f'step_delay_s must be non-negative, got {value}')
    return value

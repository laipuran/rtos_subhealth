from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Optional


class RobotInterface(ABC):
    """Abstract interface for robot motion control.

    Two implementations:
    - LowCmdRobot: drives MuJoCo simulation via unitree DDS LowCmd
    - SportClientRobot: drives real GO2 via unitree SportClient RPC
    """

    @abstractmethod
    def connect(self) -> bool:
        """Initialize DDS / SDK connection.
        Returns True on success, False on failure.
        """

    @abstractmethod
    def move(self, vx: float, vy: float, vyaw: float) -> None:
        """Velocity-based movement.
        vx: forward speed (m/s)
        vy: lateral speed (m/s)
        vyaw: angular speed (rad/s)
        """

    @abstractmethod
    def stand_up(self) -> None:
        """Transition from lying/sitting to standing position."""

    @abstractmethod
    def damp(self) -> None:
        """Stop all motion and enter damped/safe mode."""

    @abstractmethod
    def recovery_stand(self) -> None:
        """Recover from a fall back to standing."""

    @abstractmethod
    def get_pose(self) -> tuple[float, float, float]:
        """Return current robot pose as (x, y, yaw).
        Returns (0.0, 0.0, 0.0) if unavailable.
        """

    @abstractmethod
    def disconnect(self) -> None:
        """Cleanup DDS / SDK resources."""

    @abstractmethod
    def is_connected(self) -> bool:
        """Check if the underlying connection is active."""

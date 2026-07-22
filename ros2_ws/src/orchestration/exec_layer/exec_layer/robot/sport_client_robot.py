from __future__ import annotations

from typing import Optional

from unitree_sdk2py.core.channel import ChannelFactoryInitialize
from unitree_sdk2py.go2.sport.sport_client import SportClient

from .interface import RobotInterface


class SportClientRobot(RobotInterface):
    """Controls real GO2 via unitree SportClient RPC.

    Requires CycloneDDS RMW and ROS_DOMAIN_ID / interface matching
    the robot's DDS configuration.
    """

    def __init__(
        self,
        domain_id: int = 1,
        interface: str = "lo",
        timeout_s: float = 3.0,
    ) -> None:
        self._domain_id = domain_id
        self._interface = interface
        self._timeout_s = timeout_s
        self._client: Optional[SportClient] = None
        self._connected = False

    def connect(self) -> bool:
        try:
            ChannelFactoryInitialize(self._domain_id, self._interface)
        except Exception:
            pass

        self._client = SportClient(enableLease=False)
        self._client.SetTimeout(self._timeout_s)
        self._client.Init()

        # Verify connection: Hello should succeed
        try:
            code = self._client.Hello()
            if code != 0:
                return False
        except Exception:
            return False

        self._connected = True
        return True

    def move(self, vx: float, vy: float, vyaw: float) -> None:
        if self._client is None:
            return
        try:
            self._client.Move(vx, vy, vyaw)
        except Exception:
            pass

    def stand_up(self) -> None:
        if self._client is None:
            return
        try:
            self._client.StandUp()
        except Exception:
            pass

    def damp(self) -> None:
        if self._client is None:
            return
        try:
            self._client.Damp()
        except Exception:
            pass

    def recovery_stand(self) -> None:
        if self._client is None:
            return
        try:
            self._client.RecoveryStand()
        except Exception:
            pass

    def get_pose(self) -> tuple[float, float, float]:
        return (0.0, 0.0, 0.0)

    def disconnect(self) -> None:
        self._connected = False
        self._client = None

    def is_connected(self) -> bool:
        return self._connected

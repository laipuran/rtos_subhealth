# ROS Transport Interfaces

本目录是 ROS 2 `.msg`、`.srv` 和 `.action` 的唯一归属位置。它们只表达
`contracts/` 中的任务、执行、Sensor 和事件数据，不包含业务实现。

ROS generated types 必须通过 `adapters/ros-transport` 映射后才能进入 service。
不同 endpoint 可使用不同 ROS 发行版，只要实现同一 transport contract。

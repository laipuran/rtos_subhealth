# 运行时

服务端：

```bash
make run server
```

设备 endpoint：

```bash
make run endpoint DEVICE_TYPE=<device-type>
```

两种模式互斥。服务端不需要设备类型；endpoint 必须通过配置选择已注册的
adapter。Sensor 同时注册到 Orchestration 和 Execution。

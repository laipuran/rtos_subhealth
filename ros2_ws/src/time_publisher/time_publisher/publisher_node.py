#!/usr/bin/env python3
import rclpy
from rclpy.node import Node
from std_msgs.msg import String
from datetime import datetime

class TimePublisher(Node):
    def __init__(self):
        super().__init__('time_publisher')
        self.publisher_ = self.create_publisher(String, 'current_time', 10)
        timer_period = 1.0  # 秒
        self.timer = self.create_timer(timer_period, self.timer_callback)
        self.get_logger().info('Time Publisher has been started.')

    def timer_callback(self):
        now = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        msg = String()
        msg.data = now
        self.get_logger().info(f'Publishing: "{now}"')
        self.publisher_.publish(msg)

def main(args=None):
    rclpy.init(args=args)
    node = TimePublisher()
    rclpy.spin(node)
    node.destroy_node()
    rclpy.shutdown()

if __name__ == '__main__':
    main()
from setuptools import find_packages, setup

package_name = "camera_test_publisher"

setup(
    name=package_name,
    version="0.1.0",
    packages=find_packages(exclude=["test"]),
    data_files=[
        ("share/ament_index/resource_index/packages", [f"resource/{package_name}"]),
        (f"share/{package_name}", ["package.xml"]),
        (f"share/{package_name}/launch", ["launch/camera_test_publisher.launch.py"]),
        (
            f"share/{package_name}/config",
            [
                "config/camera_test_params.yaml",
                "config/cyclonedds_eth1.xml",
            ],
        ),
    ],
    install_requires=["setuptools"],
    zip_safe=True,
    maintainer="perception",
    maintainer_email="puranlai@qq.com",
    description="ROS2 test camera publisher for local built-in webcam.",
    license="MIT",
    tests_require=["pytest"],
    entry_points={
        "console_scripts": [
            "camera_test_node = camera_test_publisher.camera_test_node:main",
        ],
    },
)

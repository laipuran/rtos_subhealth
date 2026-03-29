from setuptools import find_packages, setup

package_name = "apriltag_perception"

setup(
    name=package_name,
    version="0.1.0",
    packages=find_packages(exclude=["test"]),
    data_files=[
        ("share/ament_index/resource_index/packages", [f"resource/{package_name}"]),
        (f"share/{package_name}", ["package.xml"]),
        (f"share/{package_name}/launch", ["launch/apriltag_perception.launch.py"]),
        (f"share/{package_name}/config", ["config/apriltag_params.yaml"]),
    ],
    install_requires=["setuptools"],
    zip_safe=True,
    maintainer="perception",
    maintainer_email="dev@example.com",
    description="AprilTag perception node that publishes RFC-defined detections.",
    license="MIT",
    tests_require=["pytest"],
    entry_points={
        "console_scripts": [
            "apriltag_perception_node = apriltag_perception.apriltag_node:main",
        ],
    },
)

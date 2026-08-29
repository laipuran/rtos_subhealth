from setuptools import find_packages, setup

package_name = "physio_mock_publisher"

setup(
    name=package_name,
    version="0.1.0",
    packages=find_packages(exclude=["test"]),
    data_files=[
        ("share/ament_index/resource_index/packages", [f"resource/{package_name}"]),
        (f"share/{package_name}", ["package.xml"]),
        (f"share/{package_name}/launch",
         [f"launch/{f}" for f in __import__("os").listdir("launch") if f.endswith(".launch.py")]),
    ],
    install_requires=["setuptools"],
    zip_safe=True,
    maintainer="orchestration",
    maintainer_email="puranlai@qq.com",
    description="Mock physiological sensor publisher per RFC009.",
    license="MIT",
    entry_points={
        "console_scripts": [
            "physio_mock_publisher_node = physio_mock_publisher.mock_publisher_node:main",
        ],
    },
)

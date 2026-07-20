from setuptools import find_packages, setup

package_name = "mock_exec_layer"

setup(
    name=package_name,
    version="0.1.0",
    packages=find_packages(exclude=["test"]),
    data_files=[
        ("share/ament_index/resource_index/packages", [f"resource/{package_name}"]),
        (f"share/{package_name}", ["package.xml"]),
    ],
    install_requires=["setuptools"],
    zip_safe=True,
    maintainer="orchestration",
    maintainer_email="puranlai@qq.com",
    description="Mock execution layer for testing without real hardware.",
    license="MIT",
    tests_require=["pytest"],
    entry_points={
        "console_scripts": [
            "mock_exec_layer_node = mock_exec_layer.mock_exec_layer_node:main",
        ],
    },
)

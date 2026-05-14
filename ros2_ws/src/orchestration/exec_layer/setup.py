from setuptools import find_packages, setup

package_name = "exec_layer"

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
    description="Execution layer action server skeleton per RFC003/004.",
    license="MIT",
    tests_require=["pytest"],
    entry_points={
        "console_scripts": [
            "exec_layer_node = exec_layer.exec_layer_node:main",
        ],
    },
)

from setuptools import find_packages, setup

package_name = "desc_layer"

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
    description="Description layer HTTP/WS gateway per RFC005.",
    license="MIT",
    tests_require=["pytest"],
    entry_points={
        "console_scripts": [
            "desc_layer_node = desc_layer.desc_layer_node:main",
        ],
    },
)

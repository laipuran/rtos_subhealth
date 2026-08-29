from setuptools import find_packages, setup
import os

package_name = "diagnosis_layer"

setup(
    name=package_name,
    version="0.1.0",
    packages=find_packages(exclude=["test", "tests"]),
    data_files=[
        ("share/ament_index/resource_index/packages", [f"resource/{package_name}"]),
        (f"share/{package_name}", ["package.xml"]),
    ],
    install_requires=["setuptools"],
    zip_safe=True,
    maintainer="orchestration",
    maintainer_email="puranlai@qq.com",
    description="Diagnosis layer RAG retriever per RFC009.",
    license="MIT",
    tests_require=["pytest"],
    entry_points={
        "console_scripts": [
            "diagnosis_layer_node = diagnosis_layer.diagnosis_layer_node:main",
        ],
    },
)

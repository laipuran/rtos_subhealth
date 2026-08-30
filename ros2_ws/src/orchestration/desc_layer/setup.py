from setuptools import find_packages, setup
import os

package_name = "desc_layer"
pkg_dir = os.path.dirname(os.path.abspath(__file__))

launch_dir = os.path.join(pkg_dir, "launch")
launch_files = [
    os.path.join("launch", f) for f in os.listdir(launch_dir) if f.endswith(".launch.py")
] if os.path.isdir(launch_dir) else []

data_files = [
    ("share/ament_index/resource_index/packages", [f"resource/{package_name}"]),
    (f"share/{package_name}", ["package.xml"]),
]
if launch_files:
    data_files.append(
        (os.path.join("share", package_name, "launch"), launch_files)
    )

setup(
    name=package_name,
    version="0.1.0",
    packages=find_packages(exclude=["test"]),
    data_files=data_files,
    install_requires=["setuptools", "flask>=1.0", "flask-sock>=0.7"],
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

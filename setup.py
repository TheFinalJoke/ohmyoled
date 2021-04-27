from setuptools import find_packages, setup

setup(
    name="ohmyoled",
    version="0.1.0",
    packages=find_packages(include=["lib"]),
    description="64x32 Oled Matrix Display",
    author="thefinaljoke",
    license="MIT",
    install_requires=[],
    setup_requires=["pytest-runner"],
    tests_require=["pytest==4.4.1"],
    test_suite="tests",
)
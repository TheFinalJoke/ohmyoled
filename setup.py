from setuptools import find_packages, setup

setup(
    name="ohmyoled",
    version="0.1.0",
    python_requires='<3.9.0',
    packages=find_packages(),
    description="64x32 Oled Matrix Display",
    author="thefinaljoke",
    install_requires=[
        "wheel",
        "numpy==1.20.3",
        "env-canada==0.0.35",
        "ephem==3.7.7.0",
        "fastjsonschema>=2.14.4",
        "geocoder==1.38.1",
        "gpiozero==1.5.1",
        "noaa-sdk>=0.1.18",
        "printtools==1.2",
        "PyInstaller==3.6",
        "python-tsl2591==0.2.0",
        "questionary>=1.5.2",
        "regex>=2020.4.4",
        "RPi.GPIO==0.7.0",
        "APScheduler>=3.6.3",
        "lastversion>=1.1.6",
        "nameparser==1.0.6",
        "pillow==7.1.2",
        "dbus-next",
        "aiohttp",
        "iso6709",
        ],
    setup_requires=["pytest-runner"],
    tests_require=["pytest==4.4.1"],
    test_suite="tests",
    url="https://github.com/TheFinalJoke/ohmyoled"
)

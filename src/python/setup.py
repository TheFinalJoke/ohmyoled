from setuptools import setup, find_packages

with open("ohmyoled/README.md") as file:
    ld = file.read()
setup(
    name="ohmyoled",
    version="2.2.6",
    python_requires="=3.10.15",
    py_modules=["ohmyoled"],  # Tells the name
    packages=find_packages(),
    description="64x32 Oled Matrix Display",
    author="thefinaljoke",
    long_description=ld,
    long_description_content_type="text/markdown",
    install_requires=[
        "wheel",
        "Cython",
        "numpy",
        "noaa-sdk",
        "numpy==2.1.1",
        "ephem==4.1.5",
        "fastjsonschema>=2.20.0",
        "geocoder==1.38.1",
        "gpiozero==2.0.1",
        "noaa-sdk>=0.1.21",
        "printtools==2.0.1",
        "PyInstaller==6.10.0",
        "python-tsl2591==0.2.0",
        "questionary>=2.0.1",
        "regex>=2020.9.11",
        "RPi.GPIO==0.7.1",
        "APScheduler>=3.10.4",
        "lastversion>=3.5.6",
        "nameparser==1.1.3",
        "pillow>=10.4.0",
        "dbus-next",
        "aiohttp",
        "iso6709",
        "sportsipy",
        "wget",
        "suntime",
        "ipinfo",
    ],
    extras_require={
        "dev": [
            "pytest==6.2.5",
            "twine",
        ],
    },
    url="https://github.com/TheFinalJoke/ohmyoled",
)

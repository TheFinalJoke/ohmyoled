from setuptools import setup, find_packages

with open("ohmyoled/README.md") as file:
    ld = file.read()
setup(
    name="ohmyoled",
    version="2.2.6",
    python_requires=">=3.8.9",
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
        "env-canada==0.0.35",
        "ephem==3.7.7.0",
        "fastjsonschema>=2.14.4",
        "geocoder==1.38.1",
        "gpiozero==1.5.1",
        "noaa-sdk>=0.1.18",
        "printtools==1.2",
        "Pyinstaller",
        "python-tsl2591==0.2.0",
        "questionary>=1.5.2",
        "regex>=2020.4.4",
        "RPi.GPIO>=0.7.0",
        "APScheduler>=3.6.3",
        "lastversion>=1.1.6",
        "nameparser==1.0.6",
        "pillow>=8.0.0",
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

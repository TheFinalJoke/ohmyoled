#!/usr/bin/env python3

import logging
import sys
import os
from typing import Dict
import wget
import shutil
import tarfile

from difflib import Differ
from getpass import getuser
from ohmyoled.lib.asynclib import run_async_command

class UpgradeClassException(Exception):
    pass

class Upgrader():
    def __init__(self, args, version, logger):
        self.args = args
        self.logger = logger
        self.version = version
        self.logger.setLevel(logging.DEBUG)

    def normalize_tag(self, git_stdout: str) -> str:
        return git_stdout.split()[-1].split('/')[-1]
    
    async def run_git_tags(self) -> str:
        process = await run_async_command(
            "git ls-remote --tags https://github.com/TheFinalJoke/ohmyoled.git"
        )
        if process['returncode'] != 0:
            raise UpgradeClassException("Failure to run github tags")
        return self.normalize_tag(process['stdout'])

    async def run_upgrade(self):
        if getuser() != 'root':
            self.logger.critical("Could not update, Please Become Root")
            sys.exit(2)
        up_to_date_tag = await self.run_git_tags()
        if up_to_date_tag == self.version:
            self.logger.info(f"OhMyOled is up to date. Version: {self.version}")
            sys.exit(0)
        else:
            self.logger.info(f"Current version: {self.version}, Most Updated Version: {up_to_date_tag}")
            self.logger.info("Removing old ohmyoled version")
            # Remove /usr/local/bin/ohmyoled old version
            os.remove("/usr/local/bin/ohmyoled")

            # Pull newest tar file
            ohmyoled_folder = f"ohmyoled-{up_to_date_tag}"
            output_dir = "/tmp/"
            full_path = output_dir + ohmyoled_folder
            # Check the conf file and update sections and lines that are not there
            wget.download(
                f"https://github.com/TheFinalJoke/ohmyoled/releases/download/{up_to_date_tag}/{ohmyoled_folder}.tar.gz",
                out=output_dir
            )
            
            tar = tarfile.open(full_path + ".tar.gz")
            tar.extractall('/tmp/')
            # copying new binary to execution folder
            shutil.copy(full_path + "/ohmyoled", '/usr/local/bin/')
            tar.close()

            self.logger.info("\nDeleting the zip file")
            os.remove(full_path + ".tar.gz")
            self.logger.info("Deleting Regular Folder")
            shutil.rmtree(full_path)
            self.logger.info("Upgrade was successful, Check the repo for updates in your config")
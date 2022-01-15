#!/usr/bin/env python3

import asyncio
import functools
from typing import List

def make_async(func):
    @functools.wraps(func)
    async def run_func_as_async(*args, loop=None, executor=None, **kwargs):
        if not loop:
            loop = asyncio.get_event_loop()
        return await loop.run_in_executor(
            executor, 
            functools.partial(
                func, *args, **kwargs
            )
        )
    return run_func_as_async

async def run_async_command(cmd: str, use_sudo: bool=None):
    if use_sudo:
        cmd = "sudo " + cmd
    process = await asyncio.create_subprocess_shell(
        cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    stdout, stderr = await process.communicate()
    return {
        "stdout": stdout.decode(),
        "stderr": stderr.decode(),
        "returncode": process.returncode
    }

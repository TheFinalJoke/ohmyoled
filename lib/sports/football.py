from lib.run import Runner

class Football(Runner):
    def __init__(self, token, config):
        self.config = config
        self.token = token
    
    def url_builder(self):
        headers = self.config['headers']
        return super().url_builder()
    
    async def run(self):
        pass 
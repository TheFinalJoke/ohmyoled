import unittest
from mock import patch
from lib.run import Runner

class TestRunner(unittest.TestCase):
    def setUp(self):
        self.runner = Runner()
    @patch("runner.requests")
    def test_run(self, mock_req):
        mock_req.return_value = "hello"
        self.assertEqual(self.runner.run("https://stuff"), "hello")
import asyncio
import ohmyoled.lib.tester as tester
import ohmyoled.lib.stock as stock_types
import unittest.mock as mock_types

class TestStockQuote(tester.ApiTester):
    def setUp(self) -> None:
        self.config = super().config()
        self.token = self.config["stock"]["api_key"]
        self.base_url = "https://finnhub.io/api/v1/"
        self.api = stock_types.SQuote(token=self.token, config=self.config["stock"])

    def test_url_builder(self) -> None:
        url = self.base_url + f"quote?symbol=FB&token={self.token}"
        result = self.api.url_builder("fb")
        self.assertEqual(url, result)

    def test_description_builder(self) -> None:
        url = self.base_url + f"stock/profile2?symbol=FB&token={self.token}"
        result = self.api.description_url("fb")
        self.assertEqual(url, result)
    
    @mock_types.patch.object(stock_types.SQuote, "get_data")    
    def test_run_success(self, data) -> None:
        quote_data = {
            "symbol": "META",
            "description": {'country': 'US', 'currency': 'USD', 'exchange': 'NASDAQ NMS - GLOBAL MARKET', 'finnhubIndustry': 'Media', 'ipo': '2012-05-18', 'logo': 'https://static2.finnhub.io/file/publicdatany/finnhubimage/stock_logo/FB.svg', 'marketCapitalization': 255485.59578525551, 'name': 'Meta Platforms Inc', 'phone': '16506187714.0', 'shareOutstanding': 2683.67, 'ticker': 'META', 'weburl': 'https://www.facebook.com'}
            ,'c': 95.2, 'd': 2.04, 'dp': 2.1898, 'h': 97.49, 'l': 93.55, 'o': 94.33, 'pc': 93.16, 't': 1667332804,
        }
        data.return_value = quote_data
        
        result = asyncio.run(self.api.run())
        self.assertDictEqual(result, quote_data)
    
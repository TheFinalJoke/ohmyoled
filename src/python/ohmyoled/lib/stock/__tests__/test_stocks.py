import ohmyoled.lib.stock as stock_types
import ohmyoled.lib.tester as tester
import unittest.mock as mock_types
import asyncio

class TestStocks(tester.ApiTester):
    def setUp(self) -> None:
        
        self.api = stock_types.StockApi(super().config())

    @mock_types.patch.object(stock_types.StockApi, "get_data")
    def test_symbol_lookup(self, data) -> None:
        data.return_value = {'c': 95.2, 'd': 2.04, 'dp': 2.1898, 'h': 97.49, 'l': 93.55, 'o': 94.33, 'pc': 93.16, 't': 1667332804}
        result = asyncio.run(self.api.symbol_lookup("fb"))  # type: ignore
        
        self.assertEqual(result["c"], 95.2)
    
    @mock_types.patch.object(stock_types.StockApi, "get_data")
    def test_symbol_lookup_no_data(self, data) -> None:
        data.return_value = {'c': 0, 'd': None, 'dp': None, 'h': 0, 'l': 0, 'o': 0, 'pc': 0, 't': 0}
        result = asyncio.run(self.api.symbol_lookup("blah"))  # type: ignore
        
        self.assertEqual(result["c"], 0)
    
    @mock_types.patch.object(stock_types.SQuote, "run")
    def test_run(self, run_data) -> None:
        quote_data = {
            "symbol": "META",
            "description": {'country': 'US', 'currency': 'USD', 'exchange': 'NASDAQ NMS - GLOBAL MARKET', 'finnhubIndustry': 'Media', 'ipo': '2012-05-18', 'logo': 'https://static2.finnhub.io/file/publicdatany/finnhubimage/stock_logo/FB.svg', 'marketCapitalization': 255485.59578525551, 'name': 'Meta Platforms Inc', 'phone': '16506187714.0', 'shareOutstanding': 2683.67, 'ticker': 'META', 'weburl': 'https://www.facebook.com'}
            ,'c': 95.2, 'd': 2.04, 'dp': 2.1898, 'h': 97.49, 'l': 93.55, 'o': 94.33, 'pc': 93.16, 't': 1667332804,
        }
        run_data.return_value = quote_data
        
        expecting = {"Stock": {"Quote": quote_data}}
        result = asyncio.run(self.api.run())
        self.assertDictEqual(result, expecting)
    
    @mock_types.patch.object(stock_types.SQuote, "run", side_effect=Exception)
    def test_run_throws_exception(self, run_data) -> None:
        with self.assertRaises(stock_types.StockException):
            asyncio.run(self.api.run())
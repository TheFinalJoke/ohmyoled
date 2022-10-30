import ohmyoled.lib.stock as stock_types
import ohmyoled.lib.tester as tester


class TestStocks(tester.ApiTester):
    def setUp(self) -> None:
        self.api = stock_types.StockApi(self.config)

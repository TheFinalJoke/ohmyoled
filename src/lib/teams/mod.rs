
use std::collections::HashMap;
use json;

#[derive(Debug, Copy, Clone)]
pub enum SportsTypes {
    BASKETBALL,
    BASEBALL,
    FOOTBALL,
    HOCKEY,
}
impl SportsTypes {
    pub fn print_apis() {
        println!("Choose From Apis");
        println!("Basketball");
        println!("Baseball");
        println!("Football");
        println!("Hockey");
    }
    pub fn get_sport_str(&self) -> String {
        match self {
            Self::BASEBALL => "baseball".to_string(),
            Self::BASKETBALL => "basketball".to_string(),
            Self::FOOTBALL => "football".to_string(),
            Self::HOCKEY => "hockey".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Logo {
    pub name: String,
    pub sportsdb_leagueid: i32,
    pub url: String,
    pub sport: SportsTypes,
    pub shorthand: String,
    pub apisportsid: i32,
    pub sportsdbid: i32,
    pub sportsipyid: Option<i32>,
}
impl Logo {
    pub fn to_json(&self) -> json::JsonValue {
        json::object!{
            "name": self.name.to_string(),
            "sportsdb_leagueid": self.sportsdb_leagueid,
            "url": self.url.to_string(),
            "sport": self.sport.get_sport_str(),
            "shorthand": self.shorthand.to_string(),
            "apisportsid": self.apisportsid,
            "sporsdbid": self.sportsdbid,
            "sportsipyid": match self.sportsipyid {
                Some(id) => id,
                None => 0,
            },
        }
    }
}
#[derive(Debug, Clone)]
pub struct Sport{
    pub sport: SportsTypes,
    pub teams: HashMap<String, Logo>
}
impl Sport {
    pub fn build_baseball() -> Self {
        Self {
            sport: SportsTypes::BASEBALL,
            teams: HashMap::from([
                ("Arizona Diamondbacks".to_string(), Logo{name: "Arizona Diamondbacks".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/sutyqp1431251804.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "ARI".to_string(), apisportsid: 2, sportsdbid: 135267, sportsipyid: None}),
                ("Atlanta Braves".to_string(), Logo{name: "Atlanta Braves".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/yjs76e1617811496.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "ATL".to_string(), apisportsid: 3, sportsdbid: 135268, sportsipyid: None}),
                ("Baltimore Orioles".to_string(), Logo{name: "Baltimore Orioles".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/ytywvu1431257088.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "BAL".to_string(), apisportsid: 4, sportsdbid: 135251, sportsipyid: None}),
                ("Boston Red Sox".to_string(), Logo{name: "Boston Red Sox".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/stpsus1425120215.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "BOS".to_string(), apisportsid: 5, sportsdbid: 135252, sportsipyid: None}),
                ("Chicago Cubs".to_string(), Logo{name: "Chicago Cubs".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/wxbe071521892391.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "CHC".to_string(), apisportsid: 6, sportsdbid: 135269, sportsipyid: None}),
                ("Chicago White Sox".to_string(), Logo{name: "Chicago White Sox".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/yyz5dh1554140884.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "CWS".to_string(), apisportsid: 7, sportsdbid: 135253, sportsipyid: None}),
                ("Cincinnati Reds".to_string(), Logo{name: "Cincinnati Reds".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/wspusr1431538832.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "CIN".to_string(), apisportsid: 8, sportsdbid: 135270, sportsipyid: None}),
                ("Cleveland Indians".to_string(), Logo{name: "Cleveland Indians".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/fp39hu1521904440.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "CLE".to_string(), apisportsid: 9, sportsdbid: 135254, sportsipyid: None}),
                ("Colorado Rockies".to_string(), Logo{name: "Colorado Rockies".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/wvbk1d1550584627.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "COL".to_string(), apisportsid: 10, sportsdbid: 135271, sportsipyid: None}),
                ("Detroit Tigers".to_string(), Logo{name: "Detroit Tigers".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/9dib6o1554032173.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "DET".to_string(), apisportsid: 12, sportsdbid: 135255, sportsipyid: None}),
                ("Houston Astros".to_string(), Logo{name: "Houston Astros".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/miwigx1521893583.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "HOU".to_string(), apisportsid: 15, sportsdbid: 135256, sportsipyid: None}),
                ("Kansas City Royals".to_string(), Logo{name: "Kansas City Royals".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/ii3rz81554031260.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "KC".to_string(), apisportsid: 16, sportsdbid: 135257, sportsipyid: None}),
                ("Los Angeles Angels".to_string(), Logo{name: "Los Angeles Angels".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/vswsvx1432577476.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "LAA".to_string(), apisportsid: 17, sportsdbid: 135258, sportsipyid: None}),
                ("Los Angeles Dodgers".to_string(), Logo{name: "Los Angeles Dodgers".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/rrdfmw1617528853.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "LAD".to_string(), apisportsid: 18, sportsdbid: 135272, sportsipyid: None}),
                ("Miami Marlins".to_string(), Logo{name: "Miami Marlins".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/0722fs1546001701.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "MIA".to_string(), apisportsid: 19, sportsdbid: 135273, sportsipyid: None}),
                ("Milwaukee Brewers".to_string(), Logo{name: "Milwaukee Brewers".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/08kh2a1595775193.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "MIL".to_string(), apisportsid: 20, sportsdbid: 135274, sportsipyid: None}),
                ("Minnesota Twins".to_string(), Logo{name: "Minnesota Twins".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/necd5v1521905719.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "MIN".to_string(), apisportsid: 22, sportsdbid: 135259, sportsipyid: None}),
                ("New York Mets".to_string(), Logo{name: "New York Mets".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/rxqspq1431540337.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "NYM".to_string(), apisportsid: 24, sportsdbid: 135275, sportsipyid: None}),
                ("New York Yankees".to_string(), Logo{name: "New York Yankees".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/wqwwxx1423478766.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "NYY".to_string(), apisportsid: 25, sportsdbid: 135260, sportsipyid: None}),
                ("Oakland Athletics".to_string(), Logo{name: "Oakland Athletics".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/wsxtyw1432577334.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "OAK".to_string(), apisportsid: 26, sportsdbid: 135261, sportsipyid: None}),
                ("Philadelphia Phillies".to_string(), Logo{name: "Philadelphia Phillies".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/3xrldf1617528682.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "PHI".to_string(), apisportsid: 27, sportsdbid: 135276, sportsipyid: None}),
                ("Pittsburgh Pirates".to_string(), Logo{name: "Pittsburgh Pirates".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/kw6uqr1617527138.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "PIT".to_string(), apisportsid: 28, sportsdbid: 135277, sportsipyid: None}),
                ("San Diego Padres".to_string(), Logo{name: "San Diego Padres".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/6wt1cn1617527530.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "SD".to_string(), apisportsid: 30, sportsdbid: 135278, sportsipyid: None}),
                ("San Francisco Giants".to_string(), Logo{name: "San Francisco Giants".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/mq81yb1521896622.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "SF".to_string(), apisportsid: 31, sportsdbid: 135279, sportsipyid: None}),
                ("Seattle Mariners".to_string(), Logo{name: "Seattle Mariners".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/39x9ph1521903933.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "SEA".to_string(), apisportsid: 32, sportsdbid: 135262, sportsipyid: None}),
                ("St. Louis Cardinals".to_string(), Logo{name: "St. Louis Cardinals".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/uvyvyr1424003273.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "STL".to_string(), apisportsid: 33, sportsdbid: 135280, sportsipyid: None}),
                ("St.Louis Cardinals".to_string(), Logo{name: "St. Louis Cardinals".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/uvyvyr1424003273.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "STL".to_string(), apisportsid: 33, sportsdbid: 135280, sportsipyid: None}),
                ("Tampa Bay Rays".to_string(), Logo{name: "Tampa Bay Rays".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/littyt1554031623.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "TB".to_string(), apisportsid: 34, sportsdbid: 135263, sportsipyid: None}),
                ("Texas Rangers".to_string(), Logo{name: "Texas Rangers".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/qt9qki1521893151.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "TEX".to_string(), apisportsid: 35, sportsdbid: 135264, sportsipyid: None}),
                ("Toronto Blue Jays".to_string(), Logo{name: "Toronto Blue Jays".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/f9zk3l1617527686.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "TOR".to_string(), apisportsid: 36, sportsdbid: 135265, sportsipyid: None}),
                ("Washington Nationals".to_string(), Logo{name: "Washington Nationals".to_string(), sportsdb_leagueid: 4424, url: "https://www.thesportsdb.com/images/media/team/badge/wpqrut1423694764.png".to_string(), sport: SportsTypes::BASEBALL, shorthand: "WAS".to_string(), apisportsid: 37, sportsdbid: 135281, sportsipyid: None})
            ])
        }
    }
    pub fn build_football() -> Self {
        Self {
            sport: SportsTypes::FOOTBALL,
            teams: HashMap::from(
                [
                    ("Arizona Cardinals".to_string(), Logo{name: "Arizona Cardinals".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/xvuwtw1420646838.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "ARI".to_string(), apisportsid: 0, sportsdbid: 134946, sportsipyid: None}),
                    ("Atlanta Falcons".to_string(), Logo{name: "Atlanta Falcons".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/rrpvpr1420658174.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "ATL".to_string(), apisportsid: 0, sportsdbid: 134942, sportsipyid: None}),
                    ("Baltimore Ravens".to_string(), Logo{name: "Baltimore Ravens".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/einz3p1546172463.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "BAL".to_string(), apisportsid: 0, sportsdbid: 134922, sportsipyid: None}),
                    ("Buffalo Bills".to_string(), Logo{name: "Buffalo Bills".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/6pb37b1515849026.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "BUF".to_string(), apisportsid: 0, sportsdbid: 134918, sportsipyid: None}),
                    ("Carolina Panthers".to_string(), Logo{name: "Carolina Panthers".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/xxyvvy1420940478.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "CAR".to_string(), apisportsid: 0, sportsdbid: 134943, sportsipyid: None}),
                    ("Chicago Bears".to_string(), Logo{name: "Chicago Bears".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/uwtwtv1420941123.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "CHI".to_string(), apisportsid: 0, sportsdbid: 134938, sportsipyid: None}),
                    ("Cincinnati Bengals".to_string(), Logo{name: "Cincinnati Bengals".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/qqtwwv1420941670.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "CIN".to_string(), apisportsid: 0, sportsdbid: 134923, sportsipyid: None}),
                    ("Cleveland Browns".to_string(), Logo{name: "Cleveland Browns".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/squvxy1420942389.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "CLE".to_string(), apisportsid: 0, sportsdbid: 134924, sportsipyid: None}),
                    ("Dallas Cowboys".to_string(), Logo{name: "Dallas Cowboys".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/wrxssu1450018209.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "DAL".to_string(), apisportsid: 0, sportsdbid: 134934, sportsipyid: None}),
                    ("Denver Broncos".to_string(), Logo{name: "Denver Broncos".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/upsspx1421635647.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "DEN".to_string(), apisportsid: 0, sportsdbid: 134930, sportsipyid: None}),
                    ("Detroit Lions".to_string(), Logo{name: "Detroit Lions".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/lgsgkr1546168257.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "DET".to_string(), apisportsid: 0, sportsdbid: 134939, sportsipyid: None}),
                    ("Green Bay Packers".to_string(), Logo{name: "Green Bay Packers".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/rqpwtr1421434717.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "GB".to_string(), apisportsid: 0, sportsdbid: 134940, sportsipyid: None}),
                    ("Houston Texans".to_string(), Logo{name: "Houston Texans".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/wqyryy1421436627.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "HOU".to_string(), apisportsid: 0, sportsdbid: 134926, sportsipyid: None}),
                    ("Indianapolis Colts".to_string(), Logo{name: "Indianapolis Colts".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/wqqvpx1421434058.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "IND".to_string(), apisportsid: 0, sportsdbid: 134927, sportsipyid: None}),
                    ("Jacksonville Jaguars".to_string(), Logo{name: "Jacksonville Jaguars".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/0mrsd41546427902.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "JAX".to_string(), apisportsid: 0, sportsdbid: 134928, sportsipyid: None}),
                    ("Kansas City Chiefs".to_string(), Logo{name: "Kansas City Chiefs".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/936t161515847222.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "KC".to_string(), apisportsid: 0, sportsdbid: 134931, sportsipyid: None}),
                    ("Las Vegas Raiders".to_string(), Logo{name: "Las Vegas Raiders".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/xqusqy1421724291.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "OAK".to_string(), apisportsid: 0, sportsdbid: 134932, sportsipyid: None}),
                    ("Los Angeles Chargers".to_string(), Logo{name: "Los Angeles Chargers".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/wbhu3a1548320628.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "LAC".to_string(), apisportsid: 0, sportsdbid: 135908, sportsipyid: None}),
                    ("Los Angeles Rams".to_string(), Logo{name: "Los Angeles Rams".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/8e8v4i1599764614.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "LA".to_string(), apisportsid: 0, sportsdbid: 135907, sportsipyid: None}),
                    ("Miami Dolphins".to_string(), Logo{name: "Miami Dolphins".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/trtusv1421435081.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "MIA".to_string(), apisportsid: 0, sportsdbid: 134919, sportsipyid: None}),
                    ("Minnesota Vikings".to_string(), Logo{name: "Minnesota Vikings".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/qstqqr1421609163.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "MIN".to_string(), apisportsid: 0, sportsdbid: 134941, sportsipyid: None}),
                    ("New England Patriots".to_string(), Logo{name: "New England Patriots".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/xtwxyt1421431860.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "NE".to_string(), apisportsid: 0, sportsdbid: 134920, sportsipyid: None}),
                    ("New Orleans Saints".to_string(), Logo{name: "New Orleans Saints".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/nd46c71537821337.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "NO".to_string(), apisportsid: 0, sportsdbid: 134944, sportsipyid: None}),
                    ("New York Giants".to_string(), Logo{name: "New York Giants".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/vxppup1423669459.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "NYG".to_string(), apisportsid: 0, sportsdbid: 134935, sportsipyid: None}),
                    ("New York Jets".to_string(), Logo{name: "New York Jets".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/hz92od1607953467.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "NYJ".to_string(), apisportsid: 0, sportsdbid: 134921, sportsipyid: None}),
                    ("Philadelphia Eagles".to_string(), Logo{name: "Philadelphia Eagles".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/pnpybf1515852421.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "PHI".to_string(), apisportsid: 0, sportsdbid: 134936, sportsipyid: None}),
                    ("Pittsburgh Steelers".to_string(), Logo{name: "Pittsburgh Steelers".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/2975411515853129.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "PIT".to_string(), apisportsid: 0, sportsdbid: 134925, sportsipyid: None}),
                    ("San Francisco 49ers".to_string(), Logo{name: "San Francisco 49ers".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/bqbtg61539537328.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "SF".to_string(), apisportsid: 0, sportsdbid: 134948, sportsipyid: None}),
                    ("Seattle Seahawks".to_string(), Logo{name: "Seattle Seahawks".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/wwuqyr1421434817.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "SEA".to_string(), apisportsid: 0, sportsdbid: 134949, sportsipyid: None}),
                    ("Tampa Bay Buccaneers".to_string(), Logo{name: "Tampa Bay Buccaneers".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/2dfpdl1537820969.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "TB".to_string(), apisportsid: 0, sportsdbid: 134945, sportsipyid: None}),
                    ("Tennessee Titans".to_string(), Logo{name: "Tennessee Titans".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/m48yia1515847376.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "TEN".to_string(), apisportsid: 0, sportsdbid: 134929, sportsipyid: None}),
                    ("Washington Football Team".to_string(), Logo{name: "Washington".to_string(), sportsdb_leagueid: 4391, url: "https://www.thesportsdb.com/images/media/team/badge/1m3mzp1595609069.png".to_string(), sport: SportsTypes::FOOTBALL, shorthand: "WAS".to_string(), apisportsid: 0, sportsdbid: 134937, sportsipyid: None})
                ]
            )
        }
    }
    pub fn build_hockey() -> Self {
        Self {
            sport: SportsTypes::HOCKEY,
            teams: HashMap::from(
                [
                ("Anaheim Ducks".to_string(), Logo{name: "Anaheim Ducks".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/6g9t721547289240.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "ANA".to_string(), apisportsid: 670, sportsdbid: 134846, sportsipyid: None}),
                ("Arizona Coyotes".to_string(), Logo{name: "Arizona Coyotes".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/3n1yqw1635072720.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "ARI".to_string(), apisportsid: 1460, sportsdbid: 134847, sportsipyid: None}),
                ("Boston Bruins".to_string(), Logo{name: "Boston Bruins".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/vuspuq1421791546.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "BOS".to_string(), apisportsid: 673, sportsdbid: 134830, sportsipyid: None}),
                ("Buffalo Sabres".to_string(), Logo{name: "Buffalo Sabres".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/3m3jhp1619536655.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "BUF".to_string(), apisportsid: 674, sportsdbid: 134831, sportsipyid: None}),
                ("Calgary Flames".to_string(), Logo{name: "Calgary Flames".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/v8vkk11619536610.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "CGY".to_string(), apisportsid: 675, sportsdbid: 134848, sportsipyid: None}),
                ("Carolina Hurricanes".to_string(), Logo{name: "Carolina Hurricanes".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/v07m3x1547232585.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "CAR".to_string(), apisportsid: 676, sportsdbid: 134838, sportsipyid: None}),
                ("Chicago Blackhawks".to_string(), Logo{name: "Chicago Blackhawks".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/tuwyvr1422041801.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "CHI".to_string(), apisportsid: 678, sportsdbid: 134854, sportsipyid: None}),
                ("Colorado Avalanche".to_string(), Logo{name: "Colorado Avalanche".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/wqutut1421173572.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "COL".to_string(), apisportsid: 679, sportsdbid: 134855, sportsipyid: None}),
                ("Columbus Blue Jackets".to_string(), Logo{name: "Columbus Blue Jackets".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/ssytwt1421792535.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "CBJ".to_string(), apisportsid: 680, sportsdbid: 134839, sportsipyid: None}),
                ("Dallas Stars".to_string(), Logo{name: "Dallas Stars".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/qrvywq1422042125.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "DAL".to_string(), apisportsid: 681, sportsdbid: 134856, sportsipyid: None}),
                ("Detroit Red Wings".to_string(), Logo{name: "Detroit Red Wings".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/1c24ow1546544080.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "DET".to_string(), apisportsid: 682, sportsdbid: 134832, sportsipyid: None}),
                ("Edmonton Oilers".to_string(), Logo{name: "Edmonton Oilers".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/uxxsyw1421618428.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "EDM".to_string(), apisportsid: 683, sportsdbid: 134849, sportsipyid: None}),
                ("Florida Panthers".to_string(), Logo{name: "Florida Panthers".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/8qtaz11547158220.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "FLA".to_string(), apisportsid: 684, sportsdbid: 134833, sportsipyid: None}),
                ("Los Angeles Kings".to_string(), Logo{name: "Los Angeles Kings".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/uvwtvx1421535024.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "LAK".to_string(), apisportsid: 685, sportsdbid: 134852, sportsipyid: None}),
                ("Minnesota Wild".to_string(), Logo{name: "Minnesota Wild".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/swtsxs1422042685.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "MIN".to_string(), apisportsid: 687, sportsdbid: 134857, sportsipyid: None}),
                ("Montreal Canadiens".to_string(), Logo{name: "Montreal Canadiens".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/stpryx1421791753.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "MTL".to_string(), apisportsid: 688, sportsdbid: 134834, sportsipyid: None}),
                ("Nashville Predators".to_string(), Logo{name: "Nashville Predators".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/twqyvy1422052908.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "NSH".to_string(), apisportsid: 689, sportsdbid: 134858, sportsipyid: None}),
                ("New Jersey Devils".to_string(), Logo{name: "New Jersey Devils".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/z4rsvp1619536740.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "NJD".to_string(), apisportsid: 690, sportsdbid: 134840, sportsipyid: None}),
                ("New York Islanders".to_string(), Logo{name: "New York Islanders".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/hqn8511619536714.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "NYI".to_string(), apisportsid: 691, sportsdbid: 134841, sportsipyid: None}),
                ("New York Rangers".to_string(), Logo{name: "New York Rangers".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/bez4251546192693.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "NYR".to_string(), apisportsid: 692, sportsdbid: 134842, sportsipyid: None}),
                ("Ottawa Senators".to_string(), Logo{name: "Ottawa Senators".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/2tc1qy1619536592.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "OTT".to_string(), apisportsid: 693, sportsdbid: 134835, sportsipyid: None}),
                ("Philadelphia Flyers".to_string(), Logo{name: "Philadelphia Flyers".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/qxxppp1421794965.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "PHI".to_string(), apisportsid: 695, sportsdbid: 134843, sportsipyid: None}),
                ("Pittsburgh Penguins".to_string(), Logo{name: "Pittsburgh Penguins".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/dsj3on1546192477.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "PIT".to_string(), apisportsid: 696, sportsdbid: 134844, sportsipyid: None}),
                ("San Jose Sharks".to_string(), Logo{name: "San Jose Sharks".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/yui7871546193006.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "SJS".to_string(), apisportsid: 697, sportsdbid: 134853, sportsipyid: None}),
                ("Seattle Kraken".to_string(), Logo{name: "Seattle Kraken".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/zsx49m1595775836.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "".to_string(), apisportsid: 1436, sportsdbid: 140082, sportsipyid: None}),
                ("St. Louis Blues".to_string(), Logo{name: "St. Louis Blues".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/rsqtwx1422053715.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "STL".to_string(), apisportsid: 698, sportsdbid: 134859, sportsipyid: None}),
                ("Tampa Bay Lightning".to_string(), Logo{name: "Tampa Bay Lightning".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/swysut1421791822.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "TBL".to_string(), apisportsid: 699, sportsdbid: 134836, sportsipyid: None}),
                ("Toronto Maple Leafs".to_string(), Logo{name: "Toronto Maple Leafs".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/mxig4p1570129307.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "TOR".to_string(), apisportsid: 700, sportsdbid: 134837, sportsipyid: None}),
                ("Vancouver Canucks".to_string(), Logo{name: "Vancouver Canucks".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/xqxxpw1421875519.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "VAN".to_string(), apisportsid: 701, sportsdbid: 134850, sportsipyid: None}),
                ("Vegas Golden Knights".to_string(), Logo{name: "Vegas Golden Knights".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/7fd4521619536689.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "VGK".to_string(), apisportsid: 702, sportsdbid: 135913, sportsipyid: None}),
                ("Washington Capitals".to_string(), Logo{name: "Washington Capitals".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/u17iel1547157581.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "WSH".to_string(), apisportsid: 703, sportsdbid: 134845, sportsipyid: None}),
                ("Winnipeg Jets".to_string(), Logo{name: "Winnipeg Jets".to_string(), sportsdb_leagueid: 4380, url: "https://www.thesportsdb.com/images/media/team/badge/bwn9hr1547233611.png".to_string(), sport: SportsTypes::HOCKEY, shorthand: "WPG".to_string(), apisportsid: 704, sportsdbid: 134851, sportsipyid: None})
                ]
            )
        }
    }
    pub fn build_basketball() -> Self {
        Self {
            sport: SportsTypes::BASKETBALL,
            teams: HashMap::from([
                ("Atlanta Hawks".to_string(), Logo{name: "Atlanta Hawks".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/q3bx641635067495.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "ATL".to_string(), apisportsid: 132, sportsdbid: 134880, sportsipyid: None}),
                ("Boston Celtics".to_string(), Logo{name: "Boston Celtics".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/051sjd1537102179.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "BOS".to_string(), apisportsid: 133, sportsdbid: 134860, sportsipyid: None}),
                ("Brooklyn Nets".to_string(), Logo{name: "Brooklyn Nets".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/h0dwny1600552068.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "BKN".to_string(), apisportsid: 134, sportsdbid: 134861, sportsipyid: None}),
                ("Charlotte Hornets".to_string(), Logo{name: "Charlotte Hornets".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/xqtvvp1422380623.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "CHA".to_string(), apisportsid: 135, sportsdbid: 134881, sportsipyid: None}),
                ("Chicago Bulls".to_string(), Logo{name: "Chicago Bulls".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/yk7swg1547214677.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "CHI".to_string(), apisportsid: 136, sportsdbid: 134870, sportsipyid: None}),
                ("Cleveland Cavaliers".to_string(), Logo{name: "Cleveland Cavaliers".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/a2pp4c1503741152.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "CLE".to_string(), apisportsid: 137, sportsdbid: 134871, sportsipyid: None}),
                ("Dallas Mavericks".to_string(), Logo{name: "Dallas Mavericks".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/yqrxrs1420568796.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "DAL".to_string(), apisportsid: 138, sportsdbid: 134875, sportsipyid: None}),
                ("Denver Nuggets".to_string(), Logo{name: "Denver Nuggets".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/8o8j5k1546016274.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "DEN".to_string(), apisportsid: 139, sportsdbid: 134885, sportsipyid: None}),
                ("Detroit Pistons".to_string(), Logo{name: "Detroit Pistons".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/lg7qrc1621594751.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "DET".to_string(), apisportsid: 140, sportsdbid: 134872, sportsipyid: None}),
                ("Golden State Warriors".to_string(), Logo{name: "Golden State Warriors".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/irobi61565197527.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "GSW".to_string(), apisportsid: 141, sportsdbid: 134865, sportsipyid: None}),
                ("Houston Rockets".to_string(), Logo{name: "Houston Rockets".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/yezpho1597486052.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "HOU".to_string(), apisportsid: 142, sportsdbid: 134876, sportsipyid: None}),
                ("Indiana Pacers".to_string(), Logo{name: "Indiana Pacers".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/v6jzgm1503741821.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "IND".to_string(), apisportsid: 143, sportsdbid: 134873, sportsipyid: None}),
                ("Los Angeles Clippers".to_string(), Logo{name: "Los Angeles Clippers".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/jv7tf21545916958.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "LAC".to_string(), apisportsid: 144, sportsdbid: 134866, sportsipyid: None}),
                ("Los Angeles Lakers".to_string(), Logo{name: "Los Angeles Lakers".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/spa6c11621594682.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "LAL".to_string(), apisportsid: 145, sportsdbid: 134867, sportsipyid: None}),
                ("Memphis Grizzlies".to_string(), Logo{name: "Memphis Grizzlies".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/m64v461565196789.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "MEM".to_string(), apisportsid: 146, sportsdbid: 134877, sportsipyid: None}),
                ("Miami Heat".to_string(), Logo{name: "Miami Heat".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/5v67x51547214763.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "MIA".to_string(), apisportsid: 147, sportsdbid: 134882, sportsipyid: None}),
                ("Milwaukee Bucks".to_string(), Logo{name: "Milwaukee Bucks".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/olhug01621594702.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "MIL".to_string(), apisportsid: 148, sportsdbid: 134874, sportsipyid: None}),
                ("Minnesota Timberwolves".to_string(), Logo{name: "Minnesota Timberwolves".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/5xpgjg1621594771.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "MIN".to_string(), apisportsid: 149, sportsdbid: 134886, sportsipyid: None}),
                ("New Orleans Pelicans".to_string(), Logo{name: "New Orleans Pelicans".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/f341s31523700397.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "NOP".to_string(), apisportsid: 150, sportsdbid: 134878, sportsipyid: None}),
                ("New York Knicks".to_string(), Logo{name: "New York Knicks".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/wyhpuf1511810435.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "NYK".to_string(), apisportsid: 151, sportsdbid: 134862, sportsipyid: None}),
                ("Oklahoma City Thunder".to_string(), Logo{name: "Oklahoma City Thunder".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/xpswpq1422575434.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "OKC".to_string(), apisportsid: 152, sportsdbid: 134887, sportsipyid: None}),
                ("Orlando Magic".to_string(), Logo{name: "Orlando Magic".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/txuyrr1422492990.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "ORL".to_string(), apisportsid: 153, sportsdbid: 134883, sportsipyid: None}),
                ("Philadelphia 76ers".to_string(), Logo{name: "Philadelphia 76ers".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/71545f1518464849.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "PHI".to_string(), apisportsid: 154, sportsdbid: 134863, sportsipyid: None}),
                ("Phoenix Suns".to_string(), Logo{name: "Phoenix Suns".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/qrtuxq1422919040.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "PHX".to_string(), apisportsid: 155, sportsdbid: 134868, sportsipyid: None}),
                ("Portland Trail Blazers".to_string(), Logo{name: "Portland Trail Blazers".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/mbtzin1520794112.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "POR".to_string(), apisportsid: 156, sportsdbid: 134888, sportsipyid: None}),
                ("Sacramento Kings".to_string(), Logo{name: "Sacramento Kings".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/5d3dpz1611859587.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "SAC".to_string(), apisportsid: 157, sportsdbid: 134869, sportsipyid: None}),
                ("San Antonio Spurs".to_string(), Logo{name: "San Antonio Spurs".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/obucan1611859537.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "SAS".to_string(), apisportsid: 158, sportsdbid: 134879, sportsipyid: None}),
                ("Toronto Raptors".to_string(), Logo{name: "Toronto Raptors".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/ax36vz1635070057.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "TOR".to_string(), apisportsid: 159, sportsdbid: 134864, sportsipyid: None}),
                ("Utah Jazz".to_string(), Logo{name: "Utah Jazz".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/9p1e5j1572041084.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "UTA".to_string(), apisportsid: 160, sportsdbid: 134889, sportsipyid: None}),
                ("Washington Wizards".to_string(), Logo{name: "Washington Wizards".to_string(), sportsdb_leagueid: 4387, url: "https://www.thesportsdb.com/images/media/team/badge/rhxi9w1621594729.png".to_string(), sport: SportsTypes::BASKETBALL, shorthand: "WAS".to_string(), apisportsid: 161, sportsdbid: 134884, sportsipyid: None}),
            ])
        }
    }
}
pub fn validate(user_input: String, sport: &SportsTypes) -> Result<Logo, String> {
    let mut valid_choices = match sport {
        SportsTypes::BASEBALL => Sport::build_baseball(),
        SportsTypes::BASKETBALL => Sport::build_basketball(),
        SportsTypes::FOOTBALL => Sport::build_football(),
        SportsTypes::HOCKEY => Sport::build_hockey(),            
    };
    match valid_choices.teams.remove(user_input.as_str()) {
        Some(logo) => Ok(logo),
        None => Err("That is not a valid team".to_string())
    }
}
pub fn print_teams(sport: &SportsTypes) {
    let valid_choices = match sport {
        SportsTypes::BASEBALL => Sport::build_baseball(),
        SportsTypes::BASKETBALL => Sport::build_basketball(),
        SportsTypes::FOOTBALL => Sport::build_football(),
        SportsTypes::HOCKEY => Sport::build_hockey(),            
    };
    for team in valid_choices.teams.keys(){
        println!("{}", team);
    }
}
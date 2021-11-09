from dataclasses import dataclass

@dataclass(repr=True)
class Logo:
    name: str
    abbr: str
    nba: Tuple[str, str] = None
    mlb: Tuple[str, str] = None
    nfl: Tuple[str, str] = None
    nhl: Tuple[str, str] = None


logo = {
    "DAL": Logo(
        name="Dallas",
        abbr="DAL",
        nhl=("Stars", "https://www.thesportsdb.com/images/media/team/badge/qrvywq1422042125.png"),
        nfl=("Cowboys", "https://www.thesportsdb.com/images/media/team/badge/wrxssu1450018209.png"),
        nba=("Mavericks", "https://www.thesportsdb.com/images/media/team/badge/yqrxrs1420568796.png")
    ),
    "ANA": Logo(
        name="Anaheim",
        abbr="ANA",
        nhl=("Ducks", "https://www.thesportsdb.com/images/media/team/badge/6g9t721547289240.png"),
        mlb=("Diamondbacks", "https://www.thesportsdb.com/images/media/team/badge/sutyqp1431251804.png")
    ),
    "ARI": Logo(
        name="Arizona",
        abbr="ARI",
        nhl=("Coyotes", "https://www.thesportsdb.com/images/media/team/badge/3n1yqw1635072720.png"),
        nfl=("Cardinals", "https://www.thesportsdb.com/images/media/team/badge/xvuwtw1420646838.png")
    ),
    "ATL": Logo(
        name="Atlanta",
        abbr="ATL",
        mlb=("Braves", "https://www.thesportsdb.com/images/media/team/badge/yjs76e1617811496.png"),
        nfl=("Falcons", "https://www.thesportsdb.com/images/media/team/badge/rrpvpr1420658174.png"),
        nba=("Hawks", "https://www.thesportsdb.com/images/media/team/badge/q3bx641635067495.png")
    ),
    "BAL": Logo(
        name="Baltimore",
        abbr="BAL",
        mlb=("Orioles", "https://www.thesportsdb.com/images/media/team/badge/ytywvu1431257088.png"),
        nfl=("Ravens", "https://www.thesportsdb.com/images/media/team/badge/einz3p1546172463.png")
    ),
    "BOS": Logo(
        name="Boston",
        abbr="BOS",
        nhl=("Bruins", "https://www.thesportsdb.com/images/media/team/badge/vuspuq1421791546.png"),
        mlb=("Red Sox", "https://www.thesportsdb.com/images/media/team/badge/stpsus1425120215.png"),
        nba=("Celtics", "https://www.thesportsdb.com/images/media/team/badge/051sjd1537102179.png")
    ),
    "BRO": Logo(
        name="Brooklyn",
        abbr="BRO",
        nba=("Nets", "https://www.thesportsdb.com/images/media/team/badge/h0dwny1600552068.png")
    ),
    "BUF": Logo(
        name="Buffalo",
        abbr="BUF",
        nhl=('Sabres', "https://www.thesportsdb.com/images/media/team/badge/3m3jhp1619536655.png"),
        nfl=("Bills", "https://www.thesportsdb.com/images/media/team/badge/6pb37b1515849026.png")
    ),
    "CAL": Logo(
        name="Calgary",
        abbr="CAL",
        nhl=("Flames", "https://www.thesportsdb.com/images/media/team/badge/v8vkk11619536610.png"),

    ),
    "CAR": Logo(
        name="Carolina",
        abbr="CAR",
        nhl=("Hurricanes", "https://www.thesportsdb.com/images/media/team/badge/v07m3x1547232585.png"),
        nfl=("Panthers", "https://www.thesportsdb.com/images/media/team/badge/xxyvvy1420940478.png")
    ),
    "CHI": Logo(
        name="Chicago",
        abbr="CHI",
        nhl=("Blackhawks", "https://www.thesportsdb.com/images/media/team/badge/tuwyvr1422041801.png"),
        mlb=[
            ("Cubs", "https://www.thesportsdb.com/images/media/team/badge/wxbe071521892391.png"),
            ("White Sox", "https://www.thesportsdb.com/images/media/team/badge/yyz5dh1554140884.png")
        ],
        nfl=("Bears", "https://www.thesportsdb.com/images/media/team/badge/uwtwtv1420941123.png"),
        nba=("Bulls", "https://www.thesportsdb.com/images/media/team/badge/yk7swg1547214677.png")
    ),
    "CHA": Logo(
        name="Charlotte",
        abbr="CHA",
        nba=("Hornets", "https://www.thesportsdb.com/images/media/team/badge/xqtvvp1422380623.png")
    ),
    "CIN": Logo(
        name="Cincinnati",
        abbr="CIN",
        mlb=("Reds", "https://www.thesportsdb.com/images/media/team/badge/wspusr1431538832.png"),
        nfl=("Bengals", "https://www.thesportsdb.com/images/media/team/badge/uwtwtv1420941123.png")
    ),
    "CLE": Logo(
        name="Cleveland",
        abbr="CLE",
        mlb=("Indians", "https://www.thesportsdb.com/images/media/team/badge/fp39hu1521904440.png"),
        nfl=("Browns", "https://www.thesportsdb.com/images/media/team/badge/squvxy1420942389.png"),
        nba=("Cavaliers", "https://www.thesportsdb.com/images/media/team/badge/a2pp4c1503741152.png")
    ),
    "CO": Logo(
        name="Columbus",
        abbr="CO",
        nhl=("Blue Jackets", "https://www.thesportsdb.com/images/media/team/badge/ssytwt1421792535.png")
    ),
    "COL": Logo(
        name="Colorado",
        abbr="COL",
        nhl=("Avalanche", "https://www.thesportsdb.com/images/media/team/badge/wqutut1421173572.png"),
        mlb=("Rockies", "https://www.thesportsdb.com/images/media/team/badge/wvbk1d1550584627.png")
    ),
    "DEN": Logo(
        name="Denver",
        abbr="DEN",
        nfl=("Broncos", "https://www.thesportsdb.com/images/media/team/badge/upsspx1421635647.png"),
        nba=("Nuggets", "https://www.thesportsdb.com/images/media/team/badge/8o8j5k1546016274.png")
    )
    "DET": Logo(
        name="Detroit",
        abbr="DET",
        nhl=("Red Wings", "https://www.thesportsdb.com/images/media/team/badge/1c24ow1546544080.png"),
        mlb=("Tigers", "https://www.thesportsdb.com/images/media/team/badge/9dib6o1554032173.png"),
        nfl=("Lions", "https://www.thesportsdb.com/images/media/team/badge/lgsgkr1546168257.png"),
        nba=("Pistons", "https://www.thesportsdb.com/images/media/team/badge/lg7qrc1621594751.png")
    ),
    "ED": Logo(
        name="Edmonton",
        abbr="ED",
        nhl=("Oilers", "https://www.thesportsdb.com/images/media/team/badge/uxxsyw1421618428.png")
    ),
    "FAL": Logo(
        name="Florida",
        abbr="ED",
        nhl=("Panthers", "https://www.thesportsdb.com/images/media/team/badge/8qtaz11547158220.png"),
    ),
    "GB": Logo(
        name="Green Bay",
        abbr="GB",
        nfl=("Packers", "https://www.thesportsdb.com/images/media/team/badge/lgsgkr1546168257.png")
    ),
    "GS": Logo(
        name="Golden State",
        abbr="GS",
        nba=("Warriors", "https://www.thesportsdb.com/images/media/team/badge/irobi61565197527.png")
    )
    "HOU": Logo(
        name="Houston",
        abbr="HOU",
        mlb=("Astros", "https://www.thesportsdb.com/images/media/team/badge/miwigx1521893583.png"),
        nfl=("Texans", "https://www.thesportsdb.com/images/media/team/badge/wqyryy1421436627.png"),
        nba=("Rockets", "https://www.thesportsdb.com/images/media/team/badge/yezpho1597486052.png")
    ),
    "IN": Logo(
        name="Indiana",
        abbr="IN",
        nba=("Pacers", "https://www.thesportsdb.com/images/media/team/badge/v6jzgm1503741821.png")
    )
    "IND": Logo(
        name="Indianapolis",
        abbr="IND",
        nfl=("Colts", "https://www.thesportsdb.com/images/media/team/badge/wqqvpx1421434058.png")
    ),
    "JKV": Logo(
        name="Jacksonville",
        abbr="JKV",
        nfl=("Jaguars", "https://www.thesportsdb.com/images/media/team/badge/0mrsd41546427902.png")
    ),
    "KC": Logo(
        name="Kansas City",
        abbr="KC",
        mlb=("Royals", "https://www.thesportsdb.com/images/media/team/badge/ii3rz81554031260.png"),
        nfl=("Chiefs", "https://www.thesportsdb.com/images/media/team/badge/936t161515847222.png")
    ),
    "LA": Logo(
        name="Los Angeles",
        abbr="LA",
        nhl=("Kings", "https://www.thesportsdb.com/images/media/team/badge/uvwtvx1421535024.png"),
        mlb=[
            ("Angels", "https://www.thesportsdb.com/images/media/team/badge/vswsvx1432577476.png"),
            ("Dodgers", "https://www.thesportsdb.com/images/media/team/badge/rrdfmw1617528853.png")
        ],
        nfl=[
            ("Chargers", "https://www.thesportsdb.com/images/media/team/badge/wbhu3a1548320628.png"),
            ("Rams", "https://www.thesportsdb.com/images/media/team/badge/8e8v4i1599764614.png")
        ],
        nba=[
            ("Clippers", "https://www.thesportsdb.com/images/media/team/badge/jv7tf21545916958.png"),
            ("Lakers", "https://www.thesportsdb.com/images/media/team/badge/spa6c11621594682.png")
        ]
    ),
    "MEM": Logo(
        name="Memphis",
        abbr="MEM",
        nba=("Grizzlies", "https://www.thesportsdb.com/images/media/team/badge/spa6c11621594682.png")
    ),
    "MIA": Logo(
        name="Miami",
        abbr="MIA",
        mlb=("Marlins", "https://www.thesportsdb.com/images/media/team/badge/0722fs1546001701.png"),
        nfl=("Dolphins", "https://www.thesportsdb.com/images/media/team/badge/trtusv1421435081.png"),
        nba=("Heat", "https://www.thesportsdb.com/images/media/team/badge/5v67x51547214763.png")
    ),
    "MIL": Logo(
        name="Milwaukee",
        abbr="MIL",
        mlb=("Brewers", "https://www.thesportsdb.com/images/media/team/badge/08kh2a1595775193.png"),
        nba=("Bucks", "https://www.thesportsdb.com/images/media/team/badge/olhug01621594702.png")
    ),
    "MIN": Logo(
        name="Minnesota",
        abbr="MIN",
        nhl=("Wild", "https://www.thesportsdb.com/images/media/team/badge/swtsxs1422042685.png"),
        nfl=("Vikings", "https://www.thesportsdb.com/images/media/team/badge/qstqqr1421609163.png"),
        nba=("Timberwolves", "https://www.thesportsdb.com/images/media/team/badge/5xpgjg1621594771.png")
    ),
    "MON": Logo(
        name="Montreal",
        abbr="MON",
        nhl=("Canadiens", "https://www.thesportsdb.com/images/media/team/badge/stpryx1421791753.png")
    ),
    "NAS": Logo(
        name="Nashville",
        abbr="NAS",
        nhl=("Predators", "https://www.thesportsdb.com/images/media/team/badge/twqyvy1422052908.png"),
    ),
    "NE": Logo(
        name="New England",
        abbr="NE",
        nfl=("Patriots", "https://www.thesportsdb.com/images/media/team/badge/xtwxyt1421431860.png")
    )
    "NJ": Logo(
        name="New Jersey",
        abbr="NJ",
        nhl=("Devils", "https://www.thesportsdb.com/images/media/team/badge/z4rsvp1619536740.png"),
    ),
    "NO": Logo(
        name="New Orleans",
        abbr="NO",
        nfl=("Saints", "https://www.thesportsdb.com/images/media/team/badge/nd46c71537821337.png"),
        nba=("Pelicans", "https://www.thesportsdb.com/images/media/team/badge/f341s31523700397.png")
    ),
    "NY": Logo(
        name="New York",
        abbr="NY",
        nhl=[
            ("Islanders", "https://www.thesportsdb.com/images/media/team/badge/hqn8511619536714.png"),
            ("Rangers", "https://www.thesportsdb.com/images/media/team/badge/bez4251546192693.png")
        ],
        mlb=[
            ("Mets", "https://www.thesportsdb.com/images/media/team/badge/rxqspq1431540337.png"),
            ("Yankees", "https://www.thesportsdb.com/images/media/team/badge/wqwwxx1423478766.png")
        ],
        nfl=[
            ("Giants", "https://www.thesportsdb.com/images/media/team/badge/vxppup1423669459.png"),
            ("Jets", "https://www.thesportsdb.com/images/media/team/badge/hz92od1607953467.png")
        ],
        nba=(
            "Knicks", "https://www.thesportsdb.com/images/media/team/badge/wyhpuf1511810435.png"
        )
    ),
    "OAK": Logo(
        name="Oakland",
        abbr="OAK",
        mlb=("Athletics", "https://www.thesportsdb.com/images/media/team/badge/wsxtyw1432577334.png")
    ),
    "OTT": Logo(
        name="Ottawa",
        abbr="OTT",
        nhl=("Senators", "https://www.thesportsdb.com/images/media/team/badge/2tc1qy1619536592.png")
    ),
    "OKC": Logo(
        name="Oklahoma City",
        abbr="OKC",
        nba=("Thunder", "https://www.thesportsdb.com/images/media/team/badge/xpswpq1422575434.png")
    ),
    "ORL": Logo(
        name="Orlando",
        abbr="ORL",
        nba=("Magic", "https://www.thesportsdb.com/images/media/team/badge/txuyrr1422492990.png")
    )
    "PHI": Logo(
        name="Philadelphia",
        abbr="PHI",
        nhl=("Flyers", "https://www.thesportsdb.com/images/media/team/badge/qxxppp1421794965.png"),
        mlb=("Phillies", "https://www.thesportsdb.com/images/media/team/badge/3xrldf1617528682.png"),
        nfl=("Eagles", "https://www.thesportsdb.com/images/media/team/badge/pnpybf1515852421.png"),
        nba=("76ers", "https://www.thesportsdb.com/images/media/team/badge/71545f1518464849.png")
    ),
    "PIT": Logo(
        name="Pittsburgh",
        abbr="PIT",
        nhl=("Penguins", "https://www.thesportsdb.com/images/media/team/badge/dsj3on1546192477.png"),
        mlb=("Pirates", "https://www.thesportsdb.com/images/media/team/badge/kw6uqr1617527138.png"),
        nfl=("Steelers", "https://www.thesportsdb.com/images/media/team/badge/2975411515853129.png")
    ),
    "PHE": Logo(
        name="Phoenix",
        abbr="PHE",
        nba=("Suns", "https://www.thesportsdb.com/images/media/team/badge/qrtuxq1422919040.png")
    ),
    "POR": Logo(
        name="Portland",
        abbr="POR",
        nba=("Trail Blazers", "https://www.thesportsdb.com/images/media/team/badge/mbtzin1520794112.png")
    ),
    "SCO": Logo(
        name="Sacramento",
        abbr="SCO",
        nba=("Kings", "https://www.thesportsdb.com/images/media/team/badge/5d3dpz1611859587.png")
    ),
    "SAN": Logo(
        name="San Antonio",
        abbr="SAN",
        nba=("Spurs", "https://www.thesportsdb.com/images/media/team/badge/obucan1611859537.png")
    ),
    "SD": Logo(
        name="San Diego",
        abbr="SD",
        nhl=("Padres", "https://www.thesportsdb.com/images/media/team/badge/6wt1cn1617527530.png")
    ),
    "SJ": Logo(
        name="San Jose",
        abbr="SJ",
        nhl=("Sharks", "https://www.thesportsdb.com/images/media/team/badge/yui7871546193006.png")
    ),
    "SF": Logo(
        name="San Francisco",
        abbr="SF",
        mlb=("Giants", "https://www.thesportsdb.com/images/media/team/badge/mq81yb1521896622.png"),
        nfl=("49ers", "https://www.thesportsdb.com/images/media/team/badge/bqbtg61539537328.png")
    ),
    "SEA": Logo(
        name="Seattle",
        abbr="SEA",
        nhl=("Kraken", "https://www.thesportsdb.com/images/media/team/badge/zsx49m1595775836.png"),
        mlb=("Mariners", "https://www.thesportsdb.com/images/media/team/badge/39x9ph1521903933.png"),
        nfl=("Seahawks", "https://www.thesportsdb.com/images/media/team/badge/wwuqyr1421434817.png")
    ),
    "STL": Logo(
        name="St. Louis",
        abbr="STL",
        nhl=("Blues", "https://www.thesportsdb.com/images/media/team/badge/rsqtwx1422053715.png"),
        mlb=("Cardinals", "https://www.thesportsdb.com/images/media/team/badge/uvyvyr1424003273.png")
    ),
    "TB": Logo(
        name="Tampa Bay",
        abbr="TB",
        nhl=("Lightning", "https://www.thesportsdb.com/images/media/team/badge/swysut1421791822.png"),
        mlb=("Rays", "https://www.thesportsdb.com/images/media/team/badge/littyt1554031623.png"),
        nfl=("Buccaneers", "https://www.thesportsdb.com/images/media/team/badge/2dfpdl1537820969.png")
    ),
    "TEN": Logo(
        name="Tennessee",
        abbr="TEN",
        nfl=("Titans", "https://www.thesportsdb.com/images/media/team/badge/m48yia1515847376.png")
    ),
    "TOR": Logo(
        name="Toronto",
        abbr="TOR",
        nhl=("Leafs", "https://www.thesportsdb.com/images/media/team/badge/mxig4p1570129307.png"),
        mlb=("Blue Jays", "https://www.thesportsdb.com/images/media/team/badge/f9zk3l1617527686.png"),
        nba=("Raptors", "https://www.thesportsdb.com/images/media/team/badge/ax36vz1635070057.png")
    ),
    "TEX": Logo(
        name="Texas",
        abbr="TEX",
        mlb=("Rangers", "https://www.thesportsdb.com/images/media/team/badge/qt9qki1521893151.png")
    ),
    "UT": Logo(
        name="Utah",
        abbr="UT",
        nba=("Jazz", "https://www.thesportsdb.com/images/media/team/badge/9p1e5j1572041084.png")
    ),
    "VAN": Logo(
        name="Vancouver",
        abbr="VAN",
        nhl=("Canucks", "https://www.thesportsdb.com/images/media/team/badge/xqxxpw1421875519.png")
    ),
    "VEG": Logo(
        name="Vegas",
        abbr="VEG",
        nhl=("Golden Knights", "https://www.thesportsdb.com/images/media/team/badge/7fd4521619536689.png"),
        nfl=("Raiders", "https://www.thesportsdb.com/images/media/team/badge/xqusqy1421724291.png")
    ),
    "WSH": Logo(
        name="Washington",
        abbr="WSH",
        nhl=("Capitals", "https://www.thesportsdb.com/images/media/team/badge/u17iel1547157581.png"),
        mlb=("Nationals", "https://www.thesportsdb.com/images/media/team/badge/wpqrut1423694764.png"),
        nfl=("", "https://www.thesportsdb.com/images/media/team/badge/1m3mzp1595609069.png"),
        nba=("Wizards", "https://www.thesportsdb.com/images/media/team/badge/rhxi9w1621594729.png")
    ),
    "WIN": Logo(
        name="Winnipeg",
        abbr="WIN",
        nhl=("Jets", "https://www.thesportsdb.com/images/media/team/badge/bwn9hr1547233611.png")
    ),
}
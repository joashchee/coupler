# CoffeeMUD's currencies

Studied 2026-10-26 from three places: the live game's own help (`HELP
WORLD CURRENCY`, which coffeemud.net's web server also serves as
`help.cmvp?HELPTOPIC=WORLD_CURRENCY`), the wiki's area pages (each
names its currency, `|Currency=`, and often its coins, `|ECONOMY=`),
and CoffeeMUD's source (`Libraries/BeanCounter.java`,
`Items/Basic/StdCoins.java`, `Libraries/MUDHelp.java`). Coupler reads
them in `loot.rs` (`DENOMINATIONS`, `is_money`): every coin of every
currency plays CoffeeMUD's Coins when it changes hands.

## How the game makes money

A currency belongs to an area (`MODIFY AREA`, its Currency field; the
wiki's *Currency(Builder)*). It is written as `NAME=` then its
denominations, each a value, a name and an abbreviation:
`COPPER=1 copper bit(s) (cb);10 silver bit(s) (sb);…`. A `notrade;`
after the `=` makes it unusable between players (Victory Points).

Every value is in one **base** shared by all currencies, so the
currencies trade against each other by it. The default currency's gold
coin is 1. `HELP WORLD CURRENCY` lists each currency an area uses, its
lowest denomination's rate against the base, and the others in that
lowest one ("Equal to 100 gold coins."). Without a character in a room
(the web help), it lists the world's; in the game, `HELP CURRENCY`
lists the room's area's only.

A pile of coins is named by its count and denomination, nothing else
(`StdCoins.Name` → `BeanCounter.getDenominationName`): "12 gold coins",
"1 Archon note". Only "(s)" and "(ys)" become plurals; a name without
them is the same for one or many ("5 Copper Crowns", "1 Copper
Crowns"), and "(es)" stands as it is: "3 conch(es)". Notes, bills and
dollars are paper; coins named for a metal are that metal.

A MoneyChanger (behavior) changes one currency into another at the
base rate, keeping a cut, 5% by default (`MoneyChanger.cut`). A banker
keeps only its area's currency and pays out in it (*MOBS(GenBanker)*).
Thief Appraise uses the currency where the player's recall is.

## coffeemud.net's currencies, 2026-10-26

Values in gold coins (the base). The abbreviations are what WORTH and
the shops' prices show.

| Currency | Denomination | Abbr. | In gold coins |
|---|---|---|---|
| default | gold coin(s) | | 1 |
| | golden note(s) | | 100 |
| | whole note(s) | | 10,000 |
| | Archon note(s) | | 1,000,000 |
| Alramian | Copper Crowns | CC | 0.1 |
| | Gold Crowns | GC | 10 |
| | Double Crowns | DC | 100 |
| | Royal Crowns | RC | 10,000 |
| | Archon Crown | AC | 1,000,000 |
| Merconomy | sanddollar(s) | sd | 1 |
| | shell(s) | sh | 5 |
| | conch(es) | cn | 50 |
| | nautilus(es) | nat | 5,000 |
| | archonolus(es) | arc | 1,000,000 |
| Copper | copper bit(s) | cb | 1 |
| | silver bit(s) | sb | 10 |
| | gold bit(s) | gb | 100 |
| | platinum bit(s) | pb | 500 |
| | aluminum bit(s) | ab | 10,000 |
| Troya | chalko(s) | cs | 1 |
| | obol(s) | os | 6 |
| | drachma(s) | ds | 36 |
| | mina(s) | ms | 2,160 |
| | talent(s) | ts | 129,600 |
| Weed | bud(s) | bud | 1 |
| | stem(s) | stem | 100 |
| | weed bag(s) | bag | 10,000 |
| Gold | copper piece(s) | cp | 0.01 |
| | silver piece(s) | sp | 0.1 |
| | gold piece(s) | gp | 1 |
| | platinum piece(s) | pp | 5 |
| Charcurrency | Character Coin | CH | 0.01 |
| Darkcredits | dark credit(s) | | 0.0001 |
| | dark coin(s) | | 1 |

So 1 gold coin = 10 Copper Crowns = 1 sanddollar = 1 copper bit = 1
chalko = 1 bud = 1 gold piece = 100 Character Coin = 10,000 dark
credits, before a money changer's cut. Within a currency: 1 Gold Crown
is 100 Copper Crowns; Troya counts in sixes (6 chalkos an obol, 6 obols
a drachma, 60 drachmas a mina, 60 minas a talent).

Who uses which, by the wiki's area pages (167 pages with an area box):

- **default** (gold coins): 124 areas, Midgaard, New Thalos,
  Elvenhame, Haven Lake, Aarakocran City, Tomefell and their homes
  among them.
- **Alramian**: 30, Alramus and East Alramus, Cora Isle, the Gambling
  Hall, the Valley of the Titans, Pinebaugh Territories, Sengalion, Mud
  School, Tzbrakas Training Academy, the seas, the "of Mystery" towns.
- **Merconomy**: Atlantis, Homes of Atlantis, Ocean Bottom of Mystery
  ("Shells are the primary currency").
- **Copper**: Calinth, Kang Village, the Library of Cygnus, the
  Steading of the Hill Giants.
- **Gold**: the Chessboard of Midgaard, the Dwarven Kingdom, Urgan`ali
  Dorica.
- **Troya**: Old Troya. **Weed**: Cannabis. **Charcurrency**:
  Illysia`s Cottage ("Character Coin").
- **Darkcredits**: in the game's list, but no area page names it.

Where the wiki disagrees with itself: Calinth's page says Copper (bits);
its Apartments and Townhomes are Alramian, and its Homes and Clan Homes
say default in the box but Gold Crowns in the economy line. Magthere
Wa`q's box says Alramian, its economy line Gold Pieces. The live game's help is what Coupler
follows; the names in `DENOMINATIONS` cover both either way.

## The source's other standards

`BeanCounter` defines more that no area of coffeemud.net uses on
2026-10-26, and Coupler knows their coins too in case one does:
**GOLD** and **COPPER** (above), **CREDIT** (credit(s) 1,
megacredit(s) 1,000, gigacredit(s) 1,000,000), **DOLLAR** (cent(s) 1,
dollar(s) 100, grand 100,000) and **VICTORY** (`notrade`, victory
point(s) 1). Victory Points come from Triumphs, are bound and can't be
dropped (*Victory Point*), so they never change hands as coins.

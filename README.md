# Rust Trading Tools
This app is in very early development, but will eventually facilitate the
creation of trading tools that can be used for automated trading. Currently 
only works with Kraken, as Kraken is the only API in the US that I'm aware of 
that offers free data. The candles are built from raw tick data, allowing for 
an unlimited number of candle period sizes. Kraken only returns 1000 trades 
per API request, so downloading historical data can take awhile depending on 
how far back in time you want the data to go.

A PostgreSQL database is required to use this code. Setup the database 
credentials with a .env file, or make sure that your environment has the 
following variables.
```bash
DB_HOST=db_host_name
DB_USER_NAME=db_user_name
DB_PASSWORD=db_password
```
The following samples assume that the compiled executable is named `dtrade`

## TUI Mode
A terminal user interface is available, and can be launched with the `start`
command.
```bash 
dtrade start
```
## Command Line Arguments
Otherwise, command line arguments can be passed in to do everything that the 
interface can do. To see a full list of options, pass the global `--help` flag
```bash
dtrade --help
```

### Adding and Removing Pairs
To add a new pair to the database, you can pass the `--add-pairs` flag on 
launch, followed by the exchange name, then any number of desired asset pair 
names. Do the same, but with the `--rm-pairs` flag to remove pairs from the 
database. 
```bash
dtrade database --add-pairs kraken SOLUSD ETHUSD
dtrade database --rm-pairs kraken SOLUSD ETHUSD
```
Adding pairs will create new database tables, and download some initial seed 
data, meaning there will only be 1000 trades stored in the database for each 
pair that's added via this command.  

The seed data of each pair will be offset back in time by a set amount 
according to user settings. The initial tick date offset value can be 
controlled by adjusting the `config.json` file in `crates/app_core/cache`. If 
the `config.json` file does not exist yet, then open the `config.toml` file, 
and change these values:
```toml
[data_download]
cache_size_units = 6 
cache_size_period = "M" 
```
The example values equate to 6 months, which means that the seed data for each 
newly added pair will be set back in time by 6 months. An update will need to 
be performed to fully populate the database.

### Updating Trade Data in the Database
Pass the `--update` flag on launch to update all database tables.  
```bash
dtrade database --update
```
This will download all missing trade data between the last known tick, and 
the current unix timestamp.

### Exporting Candle Data
Candles can be built via the `candles` command. Three arguments must be passed
with it to build the data (`[EXCHANGE] [TICKER] [PERIOD]`).  
```bash
dtrade candles kraken BTCUSD 1h
```

Valid time periods for candles are as follows:
| Key | Value   |
|-----|---------|
| s   | Seconds |
| m   | Minutes |
| h   | Hours   |
| d   | Days    |
| w   | Weeks   | 
| M   | Months  | 
| t   | Ticks   | 

A period value of `5m` will result in 5-minute candles, meaning that the close
date of each candle will be 5 minutes ahead of the candle before it. Tick 
based candles are also available. A period of `100t` means that each candle
contains exactly 100 ticks (trades). Weekly candles always begin on Sunday, 
and monthly candles always begin on the first of the month.

Candle data is displayed in CSV format to the terminal, and can easily be 
exported via output redirection
```bash 
dtrade candles kraken SOLUSD 50t > candles_50_tick.csv
```


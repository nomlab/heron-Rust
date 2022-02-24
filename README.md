# heron-Rust
## 説明
将来の予定発生日を予測するプログラム [heron](https://github.com/nomlab/heron) の高速化を目指し，Rust 言語で実装したプログラム

## Setup
+ clone code
  ```
  $ git clone git@github.com:nomlab/heron-Rust.git
  ```

+ compile
  ```
  cargo build
  ```
  バイナリファイル`./target/release/heron` が生成される

## Usage
```
./target/release/heron forecast [--input=INPUT] [--calendar_id=CALENDAR_ID] [--recurrence_name=RECURRENCE_NAME] [--forecast_year=FORECAST_YEAR]
```

+ INPUT
  データの入力方法を選択します．デフォルトで標準入力．`google`とすることで Google Calendar からデータを取得する．
+ CALENDAR_ID
  Google Calendar からデータを取得する場合，取得先の calendar id を指定する．
+ RECURRENCE_NAME
  Google Calendar からデータを取得する場合，取得するリカーレンス名を指定する．
+ FORECAST_YEAR
  予測年度を`YYYY-mm-dd`形式で指定する．

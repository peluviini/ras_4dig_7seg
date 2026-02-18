
## tl;dr
ras pico wでダイナミック点灯の4桁7セグを使って時刻表示させてみた。
時刻はapiをpicoから叩いて取得。

最終的な産物は↓
全てはここに書いてあります。
@[card](https://github.com/peluviini/ras_4dig_7seg)

<br>

### きっかけ
秋月の2階で見つけた50円の7セグを買ってしまったので動かしてみる。それだけ

## 7セグ
とりま型番ググってデータシートを見る。

 > [4桁7セグメントLED表示器 (秋月)](https://akizukidenshi.com/catalog/g/g115753/)

~~あれ、予期していたものと違う、
もっとなんか二進数突っ込めるのかと思っていた。~~

ちゃんと見てみると、
 > ダイナミック点灯用の4桁7セグメントLED表示器です。

と書いてあったので、ダイナミック点灯でググってみる。

どうやら、桁ごとに位相をずらして任意の数字を入れ替えていくとpwmの要領で4つ全部違う数字を表示できるらしい、

つまりこういうことだな↓
![](https://storage.googleapis.com/zenn-user-upload/6c749cbf2f5a-20260219.jpg)
コードもこんな感じにして、
```rust
loop {
    {
        seg_4.set_low().unwrap();
        d.set_low().unwrap();

        seg_1.set_high().unwrap();
        a.set_high().unwrap();
    }
    delay.delay_ms(5);
    {
        seg_1.set_low().unwrap();
        a.set_low().unwrap();

        seg_2.set_high().unwrap();
        b.set_high().unwrap();
    }
    delay.delay_ms(5);
    {
        seg_2.set_low().unwrap();
        b.set_low().unwrap();

        seg_3.set_high().unwrap();
        c.set_high().unwrap();
    }
    delay.delay_ms(5);
    {
        seg_3.set_low().unwrap();
        c.set_low().unwrap();

        seg_4.set_high().unwrap();
        d.set_high().unwrap();
    }
    delay.delay_ms(5);
}
```

> 途中で使うcrateを変えたので↑のコードは少し後述のと違う

動かしてみると...

![](https://storage.googleapis.com/zenn-user-upload/730cb2aaa3c1-20260219.jpg)
おー!うまい具合に期待通りの動きをした!
ras pico wはgpioの流せる電流が小さいらしくて適当に100Ω置いたけど、思ったより光ったね。

## api叩く
さて、7セグがいい感じに動いたので時刻の取得をやりたいですね。
まあ、適当なapi探すのが一番手っ取り早いはずなのでとりあえず、examples/でも覗きましょう。

@[card](https://github.com/embassy-rs/embassy/blob/main/examples/rp/src/bin/wifi_webrequest.rs)

いい感じのがありましたね。

apiはこれ↓を使います。
@[card](https://www.nict.go.jp/JST/JST5.html?_gl=1*3ib8mr*_ga*MjAxNTI3NzIzMC4xNzcxNDAyMjcw*_ga_GRHV5QN75N*czE3NzE0MDIyNzAkbzEkZzAkdDE3NzE0MDIyNzAkajYwJGwwJGgw*_ga_H10Z448G8R*czE3NzE0MDIyNzAkbzEkZzAkdDE3NzE0MDIyNzAkajYwJGwwJGgw)
なんか先例がページソースのhtmlから見て取ってたんで面白いしそれに倣う。
![](https://storage.googleapis.com/zenn-user-upload/c554e7f95c3f-20260219.png)

<br>

getすると、こんな感じですね。
```
~$ curl http://3fe5a5f690efc790d4764f1c528a4ebb89fa4168.nict.go.jp/cgi-bin/json
{
 "id": "ntp-a1.nict.go.jp",
 "it": 0.000,
 "st": 1771410820.312,
 "leap": 36,
 "next": 1483228800,
 "step": 1
}   
```
この`st`がUNIX時間というものらしくて決まった時刻からの経過秒数らしいです。
↓のcrateを使って、dateに変換してます。(後述)
@[card](https://docs.rs/datealgo/latest/datealgo/)


## rtcを実装する
さてrtcも実装しましょう。
@[card](https://github.com/embassy-rs/embassy/blob/main/examples/rp/src/bin/rtc.rs)

大体の実装はexamplesから持ってきてます。
先ほどの実装と合わせると、

```rust
#[allow(unused)]
#[derive(Deserialize)]
struct UnixTime<'a> {
    id: &'a str,
    it: f64,
    st: f64,
    leap: u8,
    next: u64,
    step: u8,
}

let bytes = body.as_bytes();
match from_slice::<UnixTime>(bytes) {
    Ok((unix, _used)) => {
        let mut st = unix.st;
        st += 9. * 3600.; //JST
        let (year, month, day, hour, minute, second) = { secs_to_datetime(st as i64) };
        
        let date = DateTime {
            year: year as u16,
            month: month,
            day: day,
            day_of_week: DayOfWeek::Monday, //I dont need it so no matter what its ok
            hour: hour,
            minute: minute,
            second: second,
        };
        rtc.set_datetime(date).unwrap();
    }
    Err(e) => {
        let mut buf: String<64> = String::new();
        write!(&mut buf, "Error buf: {:?}\r\n", e).ok();
        let _ = class.write_packet(buf.as_bytes()).await;
    }
}
```
曜日の実装は面倒そうだったので無視、
stの小数点を切り捨てているので少しラグができるかも...?(後付けの予言)

## 動いたもの
無限にエラーと格闘してやっと動いたはいいものの、どんなことしてたか忘れて何解説すればいいか分からないのであとは動画とリポジトリ見てください。

@[tweet](https://x.com/pelu_hasikko/status/2024141037053018404?s=20)

電源を入れるとwifiに接続→rtc動かす→rtcを7セグに表示→ボタンを押したらwebrequest→rtcに反映→表示にも反映→rtcで動き続ける
って感じの所まで動いた!!xD

何か間違い等あればよしなに。
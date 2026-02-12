
## tl;dr
ras pico wでダイナミック点灯の4桁7セグを使って時刻表示させてみた。
時刻はapiをpicoから叩いて取得。

最終的な産物は↓
()[]

<br>

### きっかけ
秋月の2階で見つけた50円の7セグを買ってしまったので動かしてみる。それだけ

## 7セグ
とりま型番ググってデータシートを見る。

 > [4桁7セグメントLED表示器 (秋月)](https://akizukidenshi.com/catalog/g/g115753/)

~~あれ,予期していたものと違う,
もっとなんか二進数突っ込めるのかと思っていた。~~

ちゃんと見てみると,:
 > ダイナミック点灯用の4桁7セグメントLED表示器です。

と書いてあったので,ダイナミック点灯でググってみる。

どうやら,桁ごとに位相をずらして任意の数字を入れ替えていくとpwmの要領で4つ全部違う数字を表示できるらしい,

つまりこういうことだな。

![]()

コードも適当に書いてみる。

```rust:main.rs(一部)
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

おー!うまい具合に期待通りの動きをした!
ras pico wはgpioの流せる電流が小さいらしくて適当に100Ω置いたけど,思ったより光ったね。

## api叩く
さて,7セグがいい感じに動いたので時刻の取得をやりたいですね。
まあ,適当なapi探すのが一番手っ取り早いはずなのでとりあえず,examples/でも覗きましょう。

@[card](https://github.com/embassy-rs/embassy/blob/main/examples/rp/src/bin/wifi_webrequest.rs)

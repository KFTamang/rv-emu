# 開発日記
## 2022/06/04
無限ループになる原因を特定した。
x0レジスタを毎回リセットしていなかった。
## 2022/06/05
ログをいい感じに表示して、デバッグの効率を上げたい。
変更があったレジスタの色を変更するとか、変更内容を表示するとか。

いくつかの命令を実装して、のこりはaddの仲間とfence, ecall, ebreak, lui, auipcくらいになってきた。
しかしテストがないのでうまく動いているか自信がない。

## 2022/06/08
ログを出す関数をまとめた。

## 2022/06/10
テスト用バイナリ生成のためにrisc-v tool chainを導入した。WSL上からだとgit clone時に改行文字がcrlfになってうまくいかなかった模様。
一時的に

```
git config --global core.autocrlf false
```

を実行して改行コード変換を無効化したらビルドに成功した。

fizzbuzzを実装したところ、さっそく実装ミスを見つけた。

## 2022/06/12
ログで変更したレジスタを色付き表示するようにした。
かなりみやすくなった。
VSCodeではデフォルトで色付き表示してくれないので、ANSI colorsというエクステンションを導入した。

## 2022/06/19
`riscv64-unknown-elf-objdump`の出力(test.dump)にうまく逆アセンブルされていない命令があった。
見ているとRV64の命令ばかりなので、RV32で逆アセンブルされているような気がする。
`-m riscv:rv64`を指定するとよいらしいので試してみると、見事に全命令が逆アセンブルできた。

## 2022/07/19
ELFファイルを読み込めるようにした。これまでは0番地からいきなり命令が始まるflat binaryしか読み込めなかったため、特殊なオプションをつけてビルドする必要があった。
これでxv6のバイナリが読み込めるようになるはず。

## 2022/07/24
CSRの中身と割込みの実装を始めた。

## 2025/01/04
久しぶりに再開。docker containerを使って開発できるようにしている。
使い方：

- gdb用コンテナイメージビルド
    - `docker build -f Dockerfile.riscv_gdb . -t riscv_gdb`
- コンテナ起動
    - `docker compose up  --remove-orphans --build`
- gdbコンテナ内でのgdb操作
    - `docker exec -it rv-emu-gdb-1 riscv64-unknown-elf-gdb /projects/apps/xv6-riscv/kernel/kernel` で起動
    - `target remote rv-emu-rust-1:9001` を実行
    - `continue` で実行開始

単独でrustコンテナを起動したい場合：
`docker compose run -it --rm rust /bin/bash`

## 2025/01/20
構成を考え直した。
各コンポーネントがバスを経由して割り込み要求を出せるように、バスへのreferenceを持たせる。
複数HartやDMACを追加することも考え、PLICは割り込み先をルーティングできるようにする必要がある。

## 2025/03/12
XV6をフォークしてsubmoduleとした。
ビルド方法：
1. `docker compose run -it --rm gdb /bin/bash`でログイン
2. `make -B`

## 2025/03/59
CSRの調査
- 0x100: sstatus
- 0x104: sie
- 0x14d: stimecmp
- 0x180: satp
- 0x300: mstatus
- 0x302: medeleg
- 0x303: mideleg
- 0x304: mie
- 0x306: mcounteren
- 0x30a: menvcfg
- 0x341: mepc
- 0x3a0: pmpcfg0
- 0x3b0: pmpaddr0
- 0xc01: time

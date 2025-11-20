# GZCTF Discord Bot

从[GZCTFBOT](github.com/CTF-Archives/GZCTFBOT)迁移而来的Discord Bot，适用于GZ::CTF的Discord赛事播报机器人。

> [!NOTE]
> 本项目只在[GZ::CTF](https://github.com/GZTimeWalker/GZCTF) v1.6.1上得到了部分测试，部分旧版本的GZ::CTF通知格式可能与本项目不兼容。

## Feature
1. 简易的消息队列，消息发送失败后自动入队等待重发，并支持写入磁盘以在程序下次启动时重发🥰
2. 使用了Discord的Embedded Link格式消息，看起来比较美观💦
3. 可通过config.toml快速配置监听的比赛😎
4. 编不出来了（

## 配置

1.  重命名 `CONFIG_TEMPLATE.toml` 为 `config.toml`。
2.  在 `config.toml` 文件中填入必要的配置信息：
    *   Discord Bot Token
    *   GZCTF API 地址
    *   用于发送通知的 Discord 频道 ID
    *   拉取消息的时间间隔
3. 编译、运行，and enjoy~



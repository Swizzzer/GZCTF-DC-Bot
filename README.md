# GZCTF Discord Bot

从[GZCTFBOT](github.com/CTF-Archives/GZCTFBOT)迁移而来的Discord Bot，适用于GZ::CTF的Discord赛事播报机器人。

## 配置

1.  重命名 `CONFIG_TEMPLATE.toml` 为 `config.toml`。
2.  在 `config.toml` 文件中填入必要的配置信息，例如：
    *   Discord Bot Token
    *   GZCTF API 地址或相关凭据
    *   用于发送通知的 Discord 频道 ID
    *   编译、运行，and enjoy~

> [!NOTE]
> 本项目只在[GZ::CTF](https://github.com/GZTimeWalker/GZCTF)v1.6.1上得到了部分测试，部分旧版本的GZ::CTF通知格式可能与本项目不兼容。
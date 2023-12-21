# 知识星球归档数据解析工具

## 归档方法

[知识星球付费内容下载脚本升级](https://mp.weixin.qq.com/s/-1iRHhJMTo4ar3F1m2T7EA)

## 编译

rustc & cargo 版本：1.75.0-nightly

正常执行 `cargo build --release` 即可编辑当前平台的可执行文件。

无法执行或不会执行的话，可以直接下载我编译好的文件，但仅有 x86 平台（可以理解为使用 Intel 芯片的电脑）的 Mac 和 Windows 版本。

## 使用方法

把可执行文件复制到归档脚本根目录下，然后分平台执行方法不同，但大同小异。

### Windows

命令行进入归档脚本的根目录中，然后执行下面脚本：

```sh
./zsxq_parser.exe -g [group_id] -m "mongodb://127.0.0.1:27017"
```

### MacOS

终端进入归档脚本的根目录中，然后执行下面脚本：

```sh
./zsxq_parser -g [group_id] -m "mongodb://127.0.0.1:27017"
```

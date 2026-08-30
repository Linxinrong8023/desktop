# 已区分插件文件与插件进程，配置存储仍需校正

## Learner response

用户说明插件安装后会落盘，即使 Ora 进程结束，安装目录中的插件仍然存在，因此“已安装但未运行”不矛盾。用户进一步提到数据目录和 `store.json`，并认为其中保存运行信息、个人信息，密钥和密码会被特殊处理。

## Demonstrated understanding

- 正确区分磁盘上的插件文件和仅在运行期间存在的进程。
- 正确理解关闭 Ora 不等于卸载插件。
- 已注意到只读插件包与可写插件数据采用不同目录。

## Corrections from current implementation

- 安装包的完整布局是 `<data-dir>/plugins/installed/<namespace>/<name>/<version>`。
- 插件全局数据位于 `<data-dir>/plugins/data/<namespace>/<name>/`。
- `store.json` 保存插件声明的配置值，不保存插件进程的 running/stopped 状态；Lifecycle 打开时插件从 stopped 开始，运行状态由进程内状态管理。
- 配置 schema v1 只支持 string、number、boolean，并将值直接序列化到 `store.json`。当前不能宣称密钥或密码会被通用安全存储特殊处理；MCP/Hook 还拒绝 `secret` 类型。

## Mastery status

“文件不等于进程”已掌握。“代码目录与数据目录为什么分离”正在学习，等待下一次主动复述验证。

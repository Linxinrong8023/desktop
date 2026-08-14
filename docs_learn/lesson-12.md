# 第 12 课：Skill 体系与 AgentDefinition（简单总结）

> 对应对话内容：Skill 体系设计、安全校验、导入会话、启动对账、AgentDefinition 角色、workflow 多 agent 协作。
> 代码地图：`crates/skill-package/`（物化+安全+校验）、`crates/application/src/skill/`（CRUD+存储）、`crates/application/src/skill_import/`（导入会话）、`crates/backend/src/skill_reconciliation.rs`（启动对账）、`crates/domain/src/skill.rs` + `agent_definition.rs`。

## 〇、核心认知

- **Skill ≈ Anthropic 的 skills**：SKILL.md + YAML front matter + 附带资源目录；概念一样，但 Ora 多了一整套**安全/校验/导入/对账机制**；
- **AgentDefinition = 角色身份**（"one configurable agent type rather than a runtime agent session"）——给每个 agent 一个身份，workflow 节点用它；
- **workflow = 多 agent 协作**：每个节点 = CLI + 模型 + Role（AgentDefinition）+ Skill + 自定义 prompt（第 15-17 课）。

## 一、Skill 体系怎么设计的

### 分工（三个 crate 各管一摊）

```
skill-package：读源 + 物化 + 安全校验（不碰数据库、不写正式目录）
application/skill：CRUD + 文件系统存储
application/skill_import：导入会话（批量安装）
```

### 落库模型很轻

```rust
pub struct Skill { id, name（ASCII slug）, description（≤4096B）, audit_fields }
```

**数据库只存名字+描述，内容在文件系统**（正式技能目录）。skill-package 的 README 明说：它只"物化和验证源快照"，不落库、不拥有会话生命周期、不决定传输语义。

### 导入会话（两阶段）

```
prepare（只读：物化+校验+扫描+manifest+查重名）→ preview（ready/conflict/invalid + 用户决定）
  → commit（冻结决定 + 后台任务：staged + 原子 promote + journal）→ cancel
```

commit 关键：**每个技能独立事务**（staged 暂存 → 文件+数据库一起原子生效，同文件系统 rename），**一个失败不连坐兄弟**；已 commit 不可取消，同决定重试幂等、不同决定 `already_committed`。

## 二、做了哪些事情（安全校验）

### 上传链路的关卡

```
① 源：.zip/.skill/.tar.gz/.tgz 或文件夹；混合源/多压缩包 → 拒绝
② 资源限制：原始 ≤50MiB、条目 ≤5000、解压 ≤200MiB、解压比预算（防 zip bomb）、
   技能 ≤500、每技能文件 ≤1000、manifest ≤1MiB
③ 路径校验（写前全部通过）：控制字符/绝对/盘符/UNC/.. /超长段/超深/非UTF-8 → 拒绝
④ Unicode 冲突：NFC 归一化 + 大小写折叠（File.txt vs file.txt、é 组合/预组合）
⑤ 扫描：精确 SKILL.md + 最近 manifest 归属规则（子技能切断父子树）
⑥ manifest：UTF-8 + 首行 --- + YAML 有效 + name slug + description ≤4096B
   （候选级错误不连坐兄弟）
```

### 防的攻击

zip-slip（路径穿越）、zip bomb（解压比爆炸）、加密压缩包、链接指向包外、控制字符路径。

### 启动对账（4 步）

```
① 恢复/清理中断事务的 journal（prepared → 回滚；committed → 收尾）
② 清理遗留暂存目录
③ 清理孤儿正式目录
④ 【有记录没文件/没 SKILL.md → 阻塞启动】（integrity over availability）
```

## 三、AgentDefinition 怎么用的、怎么设计的

```rust
/// Represents one configurable agent type rather than a runtime agent session.
pub struct AgentDefinition { id, name（身份名，稳定查找）, description, content（身份/职责提示）, audit_fields }
```

- **设计定位**：可配置的 **agent 类型/角色**——不是运行时的会话（运行时是第 7 课的 RuntimeActor）；
- **怎么用**：workflow 节点的 agentConfig 里（CLI + 模型 + Role + Skill + 自定义 prompt，第 17 课）——给节点"选一个身份"；
- **怎么设计**：`content` 就是角色的身份/职责描述（系统提示）；name 做稳定查找（trim 归一化）。

## 四、观察项

⚠️ **启动阻塞 UX 欠佳**（用户提出）：手动删了 skills 目录 → 启动失败（`SkillStorageReconciliationError::Inconsistent`），前端大概率只显示"后端启动失败"，用户不知道原因。当前是**有意的取舍**（integrity over availability：不服务坏技能），但**呈现层面确实欠佳**。改进方向：① 可操作错误（告诉用户哪个技能缺 + 修复按钮）② 优雅降级（标 broken 不阻塞）③ 自动清理（静默丢数据，危险）——① 最平衡。待后续讨论。

## 五、术语表新增

Skill Package（技能包）、SKILL.md / Front Matter（前置元数据）、zip-slip（路径穿越）、zip bomb（解压炸弹）、Nearest-Manifest Rule（最近 manifest 归属）、Import Session（导入会话）、prepare/preview/commit/cancel（两阶段会话）、Staged + Atomic Promote（暂存 + 原子生效）、Journal（事务日志）、Skill Reconciliation（启动对账）、AgentDefinition（Agent 定义/角色）、AgentConfig（节点配置）。详见桌面 software technical terms.md。

## 六、下一课预告

> 第 13 课：Spec 管理与租约（ProjectWorkContext）——Spec 目录怎么发现和索引？窗口租约机制怎么工作？scheduler 干什么？（PWC 租约目前未接线，标注"设计 vs 现状"）

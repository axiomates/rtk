# Axiomate 分支维护手册

`axiomate` 分支是给 Axiomate 使用的 RTK 变体。Axiomate 位于
`C:\public\tools\axiomate`，依赖这个分支提供输出节流能力。

`master` 是上游主分支；`axiomate` 需要定期合并 `master` 的发布和修复。
合并时必须保留本分支的两个核心差异：

1. RTK 自己生成的提示、warning、parser degradation 诊断默认不能写入 stderr。
2. Axiomate 不使用 Claude hook 管线，`rtk rewrite` 必须保持纯 rewrite 服务。

这些差异最早集中来自提交 `33041c41b165305600e1f67db4886d82c78303bf`
(`Prepare rtk axiomate quiet runtime`)。

## 分支不变量

### 1. RTK 提示默认静默

`src/core/advisory.rs` 是本分支的提示输出门控：

```rust
pub fn enabled() -> bool {
    std::env::var("RTK_ADVISORY").as_deref() == Ok("1")
        || std::env::var("RTK_VERBOSE_WARNINGS").as_deref() == Ok("1")
}
```

所有 RTK 自己生成的非必要提示都应该使用：

```rust
crate::advisory_eprintln!("...");
```

默认情况下这些输出保持静默。调试时可以显式打开：

```powershell
$env:RTK_ADVISORY = "1"
```

或：

```powershell
$env:RTK_VERBOSE_WARNINGS = "1"
```

需要门控的典型内容：

- `[rtk] warning: ...`
- parser degraded / passthrough 诊断
- project/global TOML filter 解析 warning
- trust 状态 warning
- stream capture cap / filter panic warning
- 权限配置解析失败这类 RTK 自己的提示

不要盲目门控这些内容：

- 子进程真实 stderr 的原样透传
- 命令失败时用户需要看到的工具错误
- 明确由用户执行的交互式/meta 命令输出，例如 `rtk init`、`rtk verify`、`rtk telemetry`

判断标准：如果文本是 RTK 在“建议、提示、解释自己行为”，用
`advisory_eprintln!`。如果文本是被包装命令本身的 stderr 语义输出，保留原路径。

### 2. 不使用 Claude hook 管线

Axiomate 直接嵌入 RTK 作为紧凑输出命令，不依赖 Claude Code hook。

`src/main.rs` 中不要恢复这些运行时检查：

```rust
hooks::hook_check::maybe_warn();
hooks::integrity::runtime_check()?;
```

上游 `master` 可能会在 `run_cli()` 里重新引入 hook 安装状态 warning 或
runtime integrity check。合并时必须保持 axiomate 分支的行为：

- operational command 不因为 hook 未安装、过期或 hash 不匹配而写 stderr。
- operational command 不因为 Claude hook 状态而失败。
- hook 相关命令本身可以保留；只是 Axiomate 默认执行路径不走它们。

### 3. `rtk rewrite` 是纯服务端点

`src/hooks/rewrite_cmd.rs` 在 axiomate 分支上不是 Claude permission resolver。
它只做 rewrite 查询：

| Exit | Stdout | 含义 |
| --- | --- | --- |
| 0 | rewritten command | 找到 RTK rewrite |
| 1 | 空 | 没有 rewrite，调用方执行原命令 |

不要恢复上游 hook 协议中的这些行为：

- 读取 Claude/Cursor/Gemini permission settings 来决定 rewrite exit code
- deny 返回 exit `2`
- ask/default 返回 exit `3`
- 输出权限提示或 hook advisory stderr

可以保留的上游安全增强：

- 对 command substitution、backtick、文件重定向等不可证明 shell 结构直接 passthrough。
- 这种 passthrough 必须静默：exit `1`，stdout/stderr 都为空。

## 常见冲突文件

合并 `master` 时优先检查这些文件：

- `src/core/mod.rs`
  - 必须保留 `pub mod advisory;`
  - 同时接受上游新增模块，例如 `pub mod args_utils;`

- `src/core/advisory.rs`
  - 文件必须存在。
  - 默认只在 `RTK_ADVISORY=1` 或 `RTK_VERBOSE_WARNINGS=1` 时输出。

- `src/main.rs`
  - 不要恢复 `hooks::hook_check::maybe_warn()`
  - 不要恢复 operational command 的 `hooks::integrity::runtime_check()?`
  - 不要恢复会写 stderr 的 pnpm filter warning；如果上游改善了功能，优先用 passthrough 或静默行为。

- `src/hooks/rewrite_cmd.rs`
  - 保留纯 rewrite 服务语义。
  - 不要调用 `check_command()` 作为 rewrite 的默认路径。
  - 可以调用 `contains_unattestable_construct()` 让不可证明命令静默 passthrough。

- `src/hooks/permissions.rs`
  - 上游权限逻辑可以保留给 hook 子命令使用。
  - 如果 axiomate 的 rewrite 不再调用某些上游函数，允许用明确的 `#[allow(dead_code)]` 保留兼容入口。
  - 权限配置解析失败 warning 应使用 `advisory_eprintln!`。

- `src/core/stream.rs`
- `src/core/toml_filter.rs`
- `src/parser/mod.rs`
- `src/hooks/trust.rs`
- `src/cmds/system/pipe_cmd.rs`
  - 这些文件中 RTK 自己的 warning/diagnostic 应走 `advisory_eprintln!`。

- `.github/workflows/axiomate-release.yml`
  - 这是 axiomate 分支专用 release workflow。合并 `master` 时不要删除。

## 合并最新 master 的标准流程

以下命令假设当前仓库是 `C:\public\workspace\rtk`。

### 1. 合并前检查

```powershell
cd C:\public\workspace\rtk
git status --short --branch
git branch --show-current
git log --oneline --decorate --max-count=10 axiomate master
```

要求：

- 当前分支是 `axiomate`
- 工作区干净
- `master` 已经是要合入的目标版本

如果需要更新远端：

```powershell
git fetch origin
git checkout master
git pull --ff-only origin master
git checkout axiomate
```

记录合并前 axiomate 头，后面做 stderr 差异审计会用到：

```powershell
$old = git rev-parse HEAD
```

### 2. 执行合并

```powershell
git merge master
```

如果有冲突，按“常见冲突文件”和“分支不变量”处理。

推荐冲突处理原则：

- 上游功能修复尽量接受。
- 与 quiet runtime 冲突的 stderr 提示改为 `advisory_eprintln!`。
- 与 Axiomate rewrite 服务冲突的 Claude permission exit-code 逻辑不要接受。
- 可以接受上游安全增强，但必须保持静默 passthrough。

冲突处理后检查：

```powershell
rg -n "^(<<<<<<<|=======|>>>>>>>)"
git status --short --branch
```

### 3. 重点代码审计

确认 quiet runtime 入口仍在：

```powershell
Test-Path src/core/advisory.rs
rg -n "pub mod advisory|advisory_eprintln|RTK_ADVISORY|RTK_VERBOSE_WARNINGS" src/core src/parser src/hooks src/cmds/system
```

确认 `main.rs` 没有恢复 hook runtime warning/check：

```powershell
rg -n "maybe_warn|runtime_check|validate_pnpm_filters|warning: --filter" src/main.rs
```

期望：

- `src/main.rs` 中没有 `hooks::hook_check::maybe_warn()`
- `src/main.rs` 中没有 operational command 的 `hooks::integrity::runtime_check()?`
- 没有 pnpm filter warning 回到 `eprintln!`

确认 rewrite 没有恢复 Claude permission exit-code 协议：

```powershell
rg -n "check_command|PermissionVerdict|RewriteOutcome|process::exit\\(2\\)|process::exit\\(3\\)" src/hooks/rewrite_cmd.rs
```

期望：

- `src/hooks/rewrite_cmd.rs` 不调用 `check_command()`
- 没有 ask/default exit `3`
- 没有 deny exit `2`
- `rtk rewrite` 只保留 exit `0` 和 exit `1`

### 4. stderr 提示审计

先扫全仓库 stderr 写入：

```powershell
rg -n "eprintln!|eprint!|writeln!\\(io::stderr|std::io::stderr|io::stderr|StandardStream::stderr|advisory_eprintln" src
```

然后只看这次合并相对旧 axiomate 新增的 stderr 写入：

```powershell
git diff --diff-filter=AM --unified=0 $old -- src |
  rg -n "^\\+.*(eprintln!|eprint!|writeln!\\(io::stderr|std::io::stderr|io::stderr|StandardStream::stderr|advisory_eprintln)"
```

逐条分类：

- RTK 自己新增的提示、warning、diagnostic：必须改成 `crate::advisory_eprintln!`。
- 子进程 stderr 原样透传：通常可以保留。
- 用户显式运行的 meta/interactive 命令输出：通常可以保留。
- 新增 hook/runtime advisory：默认不应进入 Axiomate operational path。

如果已经提交 merge commit，也可以用：

```powershell
git diff --diff-filter=AM --unified=0 HEAD^1..HEAD -- src |
  rg -n "^\\+.*(eprintln!|eprint!|writeln!\\(io::stderr|std::io::stderr|io::stderr|StandardStream::stderr|advisory_eprintln)"
```

### 5. 构建和测试

基础检查：

```powershell
cargo fmt
cargo check
cargo test hooks::rewrite_cmd
cargo test hooks::permissions
cargo test discover::registry::tests::test_rewrite_rtk_disabled_returns_none_without_stderr_contract
```

Windows 上全量测试依赖 `sh/true/false/cat/dd/tr/fold` 等 Unix 工具。
如果直接 `cargo test` 因 program not found 失败，先临时加入 Git for Windows 工具目录：

```powershell
$env:PATH = "C:\Program Files\Git\usr\bin;" + $env:PATH
cargo test
```

### 6. rewrite stderr smoke test

构建二进制：

```powershell
cargo build
```

确认普通 rewrite 只写 stdout：

```powershell
$out = Join-Path $env:TEMP "rtk-rewrite-out.txt"
$err = Join-Path $env:TEMP "rtk-rewrite-err.txt"
Remove-Item -LiteralPath $out,$err -ErrorAction SilentlyContinue
.\target\debug\rtk.exe rewrite "git status" > $out 2> $err
"code=$LASTEXITCODE"
"stdout=$(Get-Content -Raw $out)"
"stderr_len=$((Get-Item $err).Length)"
```

期望：

- `code=0`
- stdout 是 `rtk git status`
- `stderr_len=0`

确认不可证明结构静默 passthrough：

```powershell
$out = Join-Path $env:TEMP "rtk-rewrite-unattestable-out.txt"
$err = Join-Path $env:TEMP "rtk-rewrite-unattestable-err.txt"
Remove-Item -LiteralPath $out,$err -ErrorAction SilentlyContinue
.\target\debug\rtk.exe rewrite 'git status $(rm -rf /tmp/x)' > $out 2> $err
"code=$LASTEXITCODE"
"stdout_len=$((Get-Item $out).Length)"
"stderr_len=$((Get-Item $err).Length)"
```

期望：

- `code=1`
- `stdout_len=0`
- `stderr_len=0`

### 7. 提交合并

确认没有未解决冲突：

```powershell
git ls-files -u
git diff --check
git status --short --branch
```

提交：

```powershell
git commit -m "merge master into axiomate"
```

最终确认：

```powershell
git status --short --branch
git log --oneline --decorate --graph --max-count=8
```

## 合并后必须能回答的问题

每次合并完成前，维护者应该能明确回答：

1. `Cargo.toml` 版本是否已跟上 `master`？
2. `src/core/advisory.rs` 是否仍存在并被 `src/core/mod.rs` 导出？
3. `rtk rewrite` 是否仍只有 exit `0` / exit `1`？
4. `rtk rewrite "git status"` 是否 stderr 为空？
5. `rtk rewrite 'git status $(...)'` 是否静默 exit `1`？
6. `src/main.rs` 是否没有恢复 hook warning / integrity runtime check？
7. 本次新增的 RTK 自有 stderr 提示是否全部改成 `advisory_eprintln!`？
8. `.github/workflows/axiomate-release.yml` 是否仍存在？

如果任一答案不是确定的“是”，不要提交或发布 axiomate 分支。

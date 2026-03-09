# lockpick

> 基于 Rust 构建的超快 JS/TS 依赖分析工具

毫秒级分析你的 JS/TS 项目依赖 — 检测未使用的包、扫描漏洞、发现重复依赖、测量依赖体积。

## 特性

- **未使用依赖检测** — 使用 [oxc](https://oxc.rs) 解析 JS/TS 源文件，找出声明但从未导入的包
- **配置文件感知** — 扫描 ESLint、Babel、PostCSS、Vite、Next.js、Webpack、Tailwind 配置文件检测插件引用（支持 JSONC 注释）
- **脚本感知** — 解析 `package.json` 脚本检测 CLI 工具（如 `tsc` → `typescript`），支持链式命令（`&&`、`||`、`;`、`|`）
- **Monorepo 支持** — 检测 pnpm/npm/yarn 工作区并独立分析每个包
- **项目配置 (.lockpickrc)** — JSON/YAML 配置文件，持久化忽略规则、语言和额外配置路径
- **漏洞扫描** — 查询 [OSV.dev](https://osv.dev) 获取已知 CVE，从向量字符串计算 CVSS 3.x 基础分数，带本地文件缓存和进度条
- **重复检测** — 查找锁文件中安装了多个版本的包
- **体积分析** — 测量 `node_modules` 中每个依赖的磁盘大小
- **许可证合规** — 从 `node_modules` 提取许可证信息，规范化 SPDX 别名，通过 `.lockpickrc` 支持允许/拒绝策略
- **自动修复** — `lockpick-cli fix` 通过包管理器移除未使用的依赖，支持 monorepo 工作区和 `--dry-run`
- **过时检测** — `lockpick-cli outdated` 检查 npm registry 获取新版本，带进度条，关联漏洞数据计算升级优先级
- **供应链安全** — `lockpick-cli supply-chain` 检测仿冒包、作用域混淆和版本异常攻击；高危/严重风险影响退出码
- **多锁文件支持** — 自动检测 pnpm-lock.yaml、bun.lock、package-lock.json 和 yarn.lock（包括 yarn Berry v2/v3/v4）
- **ESM + CJS + 动态导入** — 处理 `import`、`require()`、`require.resolve()` 和 `import()` 语法，深度 AST 遍历（if/try/class/箭头函数）
- **CI 友好** — 发现未使用依赖或漏洞时以代码 1 退出；支持 `--fail-on` 阈值和 `.lockpickrc` 阈值进行细粒度 CI 门控
- **智能 @types 关联** — 如果导入了 `react`，`@types/react` 不会被标记为未使用
- **依赖树** — `lockpick tree` 可视化完整依赖图（终端、DOT、JSON、Mermaid），支持 `--focus` 和 `--depth`
- **差异对比** — `lockpick diff <baseline.json>` 与基线对比当前状态，显示新增和已解决的问题
- **快速** — 原生 Rust 二进制，无需 Node.js 运行时
- **双语** — 英文和中文输出（`--lang zh`）
- **多种输出格式** — 终端（彩色表格）、JSON 或 Markdown（`--output <file>` 写入文件）

## 安装

### npm / pnpm / yarn

```bash
npm install -D lockpick-cli
pnpm add -D lockpick-cli
yarn add -D lockpick-cli

# 或直接运行
npx lockpick-cli
```

### 从源码构建

```bash
git clone https://github.com/Dean0801/lockpick.git
cd lockpick
cargo build --release
```

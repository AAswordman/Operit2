# 事务同步与迁移


## 一、准则

**业务修改以事务为单位原子提交，由后来修改生效；最终同步后，两个文件一致。**

两端完成迁移和本轮增量交换后，比较最终同步内容。各端以自己的导出范围检查比较是否过期；有新事务则继续增量交换。内容仍有差异时，以业务更新较新的一方为权威，强制校正并建立共同基线，再比较确认一致。

## 二、定义

| 名称 | 定义 |
| --- | --- |
| 数据接口 f | DataStore 和 SQL 共有的逻辑访问与变换接口，暴露路径、归属、字段和值，以及设置、删除、增量等操作语义。 |
| 事务 | 以 SQL 记录承载的一组原子修改，含固定身份、顺序、目标和操作；目标数据可为 DataStore 或 SQL。 |
| 修改顺序 | 因果上后发生的修改排在后面；并发修改按统一逻辑时间和稳定身份排序。文件新旧取最后业务修改的顺序。 |
| preference 迁移 | 格式迁移：转换同一数据集内部的字段、类型和编码，例如 `name → displayName`。 |
| layout 迁移 | 归属迁移：移动路径、转交模块或明确退休旧来源；本机持有执行进度。 |
| SQL 迁移 | 格式迁移：转换表结构及行的表示；数据库结构版本与业务修改版本分别记录。 |
| merge | 将两端事务按统一顺序和规则计算为一致的业务状态。 |
| merge rule | 数据所属模块声明的合并策略，包括冲突、删除、内容判同和引用处理。 |

类型、运算、迁移等价、高阶事务和收敛公式见 [事务模型](transaction-model.md)。以下情况表沿用其中的记号。

## 三、情况处理期望

格式例子：v1 的 `name` 在 v2 改为 `displayName`。记 $d=\text{displayName}$，$W_1(x,r)$、$W_2(x,r)$ 分别为两个版本对该字段写入 x、业务修订为 r 的事务；$V$ 为本机支持的格式版本。下表均指相应处理完成后的结果。

| 情况 | 文字期望 | 符号表达 |
| --- | --- | --- |
| 乱序、重复到达或离线修改 | 按统一顺序合并，每个事务效果计入一次。 | $R(\mathcal H)=R(\pi(\mathcal H\Vert\mathcal H))$ |
| $T_1$ 写 $x=x_1,y=y_1$，较新的 $T_2$ 只写 $x=x_2$ | x 取后者，y 保留前者；各事务整体发布。 | $\rho(T_1)<\rho(T_2)\Rightarrow R([T_1,T_2])[x]=(x_2,\rho(T_2)),\ R([T_1,T_2])[y]=(y_1,\rho(T_1))$ |
| A 写丙30，B 的旧格式乙20晚到 | 两端保留丙30。 | $R([W_2(\text{丙},30),W_1(\text{乙},20)])[d]=(\text{丙},30)$ |
| A 写丙30，B 在旧版写乙40后升级 | 两端采用乙40。 | $R([W_2(\text{丙},30),W_1(\text{乙},40)])[d]=(\text{乙},40)$ |
| A 写丙30，B 最后迁移甲10 | 甲仍是修订10，两端保留丙30。 | $m_F(W_1(\text{甲},10))=W_2(\text{甲},10);\ R([W_2(\text{丙},30),W_1(\text{甲},10)])[d]=(\text{丙},30)$ |
| 本地 v2 收到 v1 | 同一规则转换状态和旧事务，保留业务修订。 | $m_F(W_1(x,r))=W_2(x,r)$ |
| 收到未来格式 | 原文进入待处理，升级就绪后应用。 | $v(T)>V\Rightarrow T\in P;\ \operatorname{ready}(T)\leadsto(T\in E\land T\notin P)$ |
| 旧路径事务晚到 | 按 layout 映射交给当前归属。 | $\operatorname{loc}(T)=p_{\mathrm{old}}\Rightarrow\operatorname{loc}(m_L(T))=p_{\mathrm{new}}$ |
| layout 明确退休旧来源且迁移已完成 | 历史事务按空效果确认，账本保留原文；当前业务数据保持原值。 | $m_L(T)=\epsilon;\ \operatorname{Apply}(m_L(A),\epsilon)=m_L(A)$ |
| 新路径归属尚未识别 | 保存原文，layout 升级就绪后应用。 | $\neg\operatorname{ready}_L(T)\Rightarrow T\in P;\ \operatorname{ready}(T)\leadsto T\in E$ |
| 旧路径与旧格式同时出现 | 完成两种转换，保留原事务身份和顺序。 | $m_A(\mathcal T)=(m_F)_{m_L(A)}((m_L)_A(\mathcal T))$ |
| 漏记、漏传或漏应用 | 从原始历史、另一端或有版本依据的检查点补齐。 | $\mathcal H'=\mathcal H\cup_{\mathrm{id}}\mathcal H_{\mathrm{missing}};\ C(A_f)=C(B_f)=C(R(\mathcal H'))$ |
| 迁移或应用中断 | 按本机进度恢复，完成项保持幂等。 | $\operatorname{resume}_m(A_j,j)=m(A);\ R(\mathcal H\Vert\mathcal H)=R(\mathcal H)$ |
| 内容相同、ID 不同 | 按模块规则判同，持久映射供后续旧 ID 写入使用。 | $x\sim_\mu y\Rightarrow\alpha(\operatorname{id}(x))=\alpha(\operatorname{id}(y));\ \alpha\circ\alpha=\alpha$ |
| 引用先到、目标后到 | 保留引用，可选记忆库暂按空库读；目标到达后解析。 | $i\notin A\leadsto(\operatorname{ref}(q)=i\land\operatorname{readOptional}(i)=\varnothing);\ i\in A\leadsto\operatorname{resolve}(q)=A[i]$ |
| 删除后收到较早写入 | 保留删除事实；较新恢复由声明规则决定。 | $\rho(T_{\mathrm{write}})<\rho(T_{\mathrm{delete}})\Rightarrow R([T_{\mathrm{delete}},T_{\mathrm{write}}])[k]=\dagger$ |
| 一个数据集待处理，另一个已就绪 | 独立数据集继续提交。 | $T_a\in P\land\operatorname{ready}(T_b)\land\operatorname{owner}(T_a)\ne\operatorname{owner}(T_b)\leadsto T_b\in E$ |
| 二阶事务修改一阶事务 | 保留原文，求有效视图并重算受影响结果。 | $T_a^{\mathrm{eff}}=\operatorname{Apply}_{\mu_T}(T_a,[T_{T_a}])$ |
| 同版本、本轮增量已交换且比较未过期，内容仍不同 | 判为同步错误，选权威快照覆盖并再次比较。 | $C(A')\ne C(B')\leadsto C(A_f)=C(B_f)=C(X^*)$ |
| 校正期间出现新事务 | 使用最新历史重新同步，再比较和决定校正。 | $(\nu'_A,\nu'_B)\ne(\nu_A,\nu_B)\leadsto R(\mathcal H\cup_{\mathrm{id}}\mathcal H_{\mathrm{new}})$ |

$i\in A$ 表示身份 i 的记录存在，$\operatorname{id}$ 取记录身份；$v(T)$ 为目标数据格式版本，$\operatorname{loc}$ 为路径及归属，$\operatorname{ready}_L$ 为 layout 就绪；$x\sim_\mu y$ 表示规则判定两条记录内容相同。$T_{\mathrm{write}}$ 与 $T_{\mathrm{delete}}$ 操作同一位置 k，表中未列出的值保持原状态。

## 四、具体的细节

1. **提交路径**：SQL 保存事务原文与收发状态。业务 SQL 同库时原子提交；DataStore 或异库通过持久提交意图恢复为完整业务提交。远端保存原文后登记接收，业务原子提交后登记已应用。
2. **事务边界**：明确到 preference 文件或数据库。跨载体事务需要对应的原子提交能力；迁移保持原事务的原子边界。
3. **迁移入口**：格式迁移负责解码和结构转换，layout 迁移负责定位当前归属。旧位置与旧格式同时出现时，例如 `old.name → new.displayName`，两步都完成后应用；本机执行进度独立保存。移动文件同时转移完整检查点及历史覆盖范围，沿用原业务修订。明确退休的来源在迁移完成后映射为空效果；移动规则优先定位当前归属，未知来源保留待处理。
4. **版本职责**：业务顺序决定覆盖关系；操作版本决定解码；schema 和 layout 版本决定本机迁移；规则版本决定合并解释。
5. **迁移规则**：同一规则通过 f 作用于 DataStore、业务 SQL 及 SQL 事务视图，以迁移等价式验证完整状态、删除、增量、局部修改和分批恢复。事务转换逐步校验完整状态等式，失败时整笔保留待处理。
6. **恢复依据**：检查点保存当前值、修改范围版本、删除事实、身份映射、增量贡献及历史覆盖范围；整组记录拆分保留整组替换的范围版本，覆盖尚未出现的旧 ID。依据缺失时保留待恢复状态，恢复来源可以是其他节点或备份。
7. **比较范围**：当前对话选择属于本机状态，各端独立保存；历史误入同步的选择事务由 layout 迁移确认为空效果，保留原文及本机选择。两端完成对应格式和 layout 迁移后，以确定序列化的文件值及同结构、固定顺序导出的同步 SQL 表计算摘要。重放修订、覆盖集合和旧基线分别保存在本机账本，其差异不触发全量校正。比较范围由各端自己的本轮导出收据确定，无需两端同时采样或历史收据相同。
8. **强制校正**：内容仍不一致说明事务链路已有损坏，直接复制较新方的完整同步文件及检查点，沿用源业务修订；缺失文件一并补齐。Preference 复制完整明文后由接收端加密；SQL 复制同步表的完整结构、行及版本事实，本机账本独立保存。替换按捕获版本原子提交，期间写入则重新同步。
9. **校正落地**：两端共同保存复制结果为新基线，覆盖范围取双方已有历史的并集。已覆盖事务仅归档和确认；后续事务从新基线应用。文件与基线通过持久提交意图一起恢复，每端替换完成后校验完整复制状态及基线落盘；两端最终同步内容一致，本轮同步完成。

10. **验收依据**：业务 owner 的实际合并与操作测试逐项满足上述情况及符号定义，覆盖 DataStore、聊天 SQL、插件 SQL、两种迁移和最终强制校正；相同历史在乱序、重复、重启后得到相同业务数据。

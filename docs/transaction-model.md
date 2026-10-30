# 事务模型

用 $X\in f$ 表示 X 实现统一接口；$\sqcup$ 表示带类型标记的并集：

$$
f=f_{\mathrm{DataStore}}\sqcup f_{\mathrm{SQL}},\qquad
A,B\in f,\qquad T_a\in\operatorname{Tx}(A)\subseteq f_{\mathrm{SQL}}\subseteq f.
$$

$\operatorname{Tx}(X)$ 是以 X 为目标的事务记录集合。事务的 SQL 载体与其修改目标分别定型；f 对事务暴露目标及操作内容，外层保留身份、修改顺序和原子边界。

| 记号 | 含义 |
| --- | --- |
| $A,B;\ C(A)$ | 两端完整业务状态；$C(A)$ 是确定序列化的文件值或同步 SQL 表结构及记录；完整重放状态及旧基线另行保存。 |
| $T_a;\ \mathcal T_A=(T_{a_1},\ldots,T_{a_n})$ | 单个事务；A 端的一组事务，序列可以为空。$\rho(T_a)$ 是业务修改顺序，$\rho(A)$ 是文件最后业务修改的顺序。 |
| $\operatorname{Apply}_\mu(A,\mathcal T)$ | 按序列逐个原子应用事务；$\mu$ 是目标数据所属模块的 merge rule。 |
| $[];\ \Vert;\ \pi$ | 空序列；序列拼接；任意到达顺序的排列。 |
| $\cup_{\mathrm{id}};\ N(\mathcal H)$ | 按身份合并历史；对历史按身份去重并按 $\rho$ 排序。同身份同原文计入一次，同身份不同原文判错。 |
| $\operatorname{Eff}_{\mu_T}(\mathcal H)$ | 校验原文身份后，按目标依赖及事务规则求有效的一阶历史；仅有一阶事务时等于原历史。 |
| $m_F,m_L;\ m=m_F\circ m_L$ | 格式迁移、layout 迁移；组合表示先确定当前归属，再转换为对应格式。 |
| $m(A);\ m_A(\mathcal T)$ | 同一规则迁移完整状态；以 A 为上下文转换事务序列。与上下文无关时可简写为 $m(\mathcal T)$。 |
| $P,E;\ \operatorname{ready}(T)$ | 按身份识别的待处理、已应用事务集合；事务有效，所需格式、归属、上下文和依赖均已就绪。 |
| $X[k]=(x,r);\ \dagger$ | 位置 k 的值为 x，业务修订为 r；$\dagger$ 表示保留删除事实。 |
| $\alpha;\ \nu_X;\ \leadsto$ | 记录 ID 到规范 ID 的持久映射；X 的快照版本；经处理后到达的状态。快照版本用于发现期间写入，包括写回相同值。 |

$e_X$ 表示 X 端本轮空增量导出时的接收范围，$\widehat e_X$ 为读取最终内容后的接收范围；各端只与自己的范围比较。

完整状态写为 $A=(O,V,\rho,D,\alpha,\sigma,\Delta)$：归属与版本、值、业务修订、删除事实、身份映射、各修改范围的优先级、尚需保留的增量贡献。检查点另保存精确的历史覆盖集合 $K$。迁移等价式比较这些分量；例如 `name@50 → displayName@50` 同时转换值与范围版本，迁移后收到 `name@40` 仍由 50 生效。

字段范围独立比较优先级：父级删除 @30 后，字段写入 @20 保持删除，字段写入 @50 只重建该字段需要的对象或 ID 记录容器；同一规则用于事务应用和检查点合并。

整组记录拆为独立键时，$\sigma$ 还保存整组替换的范围版本：`cards@50 → card_*@50`。该范围同时约束未出现在快照中的 ID，修订 40 的迟到记录保持缺席，修订 60 的新记录可以加入。

**事务应用。** 空序列保持状态，拼接表示依次执行：

$$
\operatorname{Apply}_\mu(A,[])=A,\qquad
\operatorname{Apply}_\mu(A,\mathcal T\Vert[T_a])
=\operatorname{Apply}_\mu(\operatorname{Apply}_\mu(A,\mathcal T),[T_a]).
$$

**迁移等价。** 同一规则 $m$ 通过 $f$ 作用于状态和事务，须满足：

$$
m\!\left(\operatorname{Apply}_{\mu_0}(A,\mathcal T)\right)
=\operatorname{Apply}_{\mu_1}\!\left(m(A),m_A(\mathcal T)\right).
$$

$\mu_0,\mu_1$ 为迁移前后的对应规则。此式分别约束 $m_F$、$m_L$ 及其组合。原始事务保留原文；转换视图沿用身份、业务修订和原子边界，一个源事务展开的多个操作仍整体提交。需要上下文的规则使用完整基线及身份映射。

$f$ 统一可操作的表示；迁移等价式是迁移实现须满足的语义约束，由公共内核和业务测试共同检验。

layout 迁移读取旧路径和目标路径时，取 $A=(S,Q)$；$S,Q$ 各包含完整状态及历史覆盖集合 $K_S,K_Q$。记 $\ell_L$ 为归属转换、$J$ 为同归属检查点合并，则：

$$
m_L((S,Q))=J(\ell_L(S),Q),\qquad K_{m_L(A)}=K_S\cup K_Q.
$$

对修改旧路径的事务序列，原等价式具体展开为：

$$
J\!\left(\ell_L(\operatorname{Apply}_{\mu_0}(S,\mathcal T)),Q\right)
=\operatorname{Apply}_{\mu_1}\!\left(J(\ell_L(S),Q),(m_L)_{(S,Q)}(\mathcal T)\right).
$$

$J$ 的交换律、结合律、幂等律与上述迁移等价式分别验收；迁移前后业务修订均取 $S,Q$ 的最后业务修改。

layout 也可在导入及清理完成后明确退休来源。此时 $m_L$ 从当前业务视图移除旧来源，对仅修改该来源的事务取 $(m_L)_A(T)=\epsilon$，满足 $m_L(\operatorname{Apply}(A,T))=\operatorname{Apply}(m_L(A),\epsilon)=m_L(A)$。账本保留并确认原事务；尚未声明归属的来源继续待处理。

实现用同一迁移定义分别计算迁移前后状态，再对每一步事务转换执行完整状态等价检查，成功后发布转换结果；检查失败则保留原文待处理。独立数据集继续提交。具体业务规则仍由实际 owner 测试验证。

分段转换时，后一段使用前一段应用后的上下文：

$$
m_A(\mathcal T_1\Vert\mathcal T_2)
=m_A(\mathcal T_1)\Vert m_{\operatorname{Apply}_{\mu_0}(A,\mathcal T_1)}(\mathcal T_2).
$$

迁移可由本机存储提交序列 $\mathcal M_m(A)=(M_1,\ldots,M_k)$ 完成。记 $A_j=M_j(\cdots M_1(A)\cdots)$，则 $A_k=m(A)$。这些提交记录本机进度，沿用原业务修订：$\rho(m(A))=\rho(A)$。

**事务的事务。** 事务自身属于 f，因此可继续成为目标：

$$
T_a\in\operatorname{Tx}(A),\qquad
T_{T_a}\in\operatorname{Tx}(T_a)\subseteq f_{\mathrm{SQL}},\qquad
T_a^{\mathrm{eff}}=\operatorname{Apply}_{\mu_T}(T_a,[T_{T_a}]).
$$

$\mu_T$ 是事务数据的合并规则，$T_a^{\mathrm{eff}}$ 是其有效视图。递归取 $T^{(0)}=A,\ T^{(n+1)}\in\operatorname{Tx}(T^{(n)})$ 即得更高阶；各阶使用独立身份保留原文与依赖，由显式操作产生。迁移等价式中的 A 也可取 $T_a$。

启用高阶处理时，通过 $\operatorname{Eff}_{\mu_T}$ 求有效的一阶历史，再重放业务结果。变更命中基线已覆盖事务时，取可重放该事务的基线；依据暂缺则进入 P。统一接口与目标寻址应支持此能力，具体业务按需使用。

**事务同步。** 两端迁移就绪，取得共同基线 $A_0$ 及其覆盖范围后，令 $\mathcal H$ 为基线后的原始业务历史：

$$
R(\mathcal H)=\operatorname{Apply}_{\mu_1}\!\left(m(A_0),m_{A_0}(N(\operatorname{Eff}_{\mu_T}(\mathcal H)))\right),\qquad
A'=B'=R(\mathcal T_A\cup_{\mathrm{id}}\mathcal T_B).
$$

先合并原文，求有效历史，再排序、转换和重放。已迁移的基线直接使用其当前状态；$R$ 在下表中均使用相同基线、迁移和规则。

**最终强制校正。** 两端迁移和本轮增量交换完成后，以各自导出范围判断比较是否过期；新事务进入下一轮增量。全量校正的前提为：

$$
(\widehat e_A=e_A)\land(\widehat e_B=e_B)\land C(A')\ne C(B').
$$

该前提允许 $e_A\ne e_B$，无需两端同时采样。旧基线差异本身不触发全量。记 $b(X)=(X_0,K_X)$，包含完整基线状态及覆盖范围。$d(X)$ 为固定节点优先序，按业务修订、节点优先序依次比较：

$$
X^*=\underset{X\in\{A',B'\}}{\arg\max}\;(\rho(X),d(X)),\qquad
C(A_{\mathrm{final}})=C(B_{\mathrm{final}})=C(X^*).
$$

直接复制 $X^*$ 的完整同步文件，并在两端固化共同基线 $A_0^*=X^*$。令 $K_A,K_B$ 为各端校正前已处理事务的身份集合，则：

$$
K^*=K_A\cup K_B,\qquad
A_{\mathrm{next}}=B_{\mathrm{next}}=\operatorname{Apply}_{\mu_1}\!\left(A_0^*,N(\mathcal H_{\mathrm{next}}\setminus K^*)\right).
$$

$\setminus K^*$ 按事务身份排除已封存历史；其原文继续归档，后续应用以复制后的文件为起点。缺失文件按同一流程补齐。两端分别按捕获版本原子保存文件及基线，再次比较；期间写入则重新同步、选源和比较。一致后结束本轮同步。记 $\operatorname{Complete}(A,B)$ 为本轮完成，$\operatorname{ready}(A)$ 包含两种迁移、事务处理及对应校验协议均就绪，则：

$$
\operatorname{Complete}(A,B)\Rightarrow\operatorname{ready}(A)\land\operatorname{ready}(B)\land C(A)=C(B).
$$

全量校正还需验证共同基线已落盘：

$$
\operatorname{CorrectedComplete}(A,B)\Rightarrow\operatorname{Complete}(A,B)\land b(A)=b(B).
$$

强制校正保证最终内容相等；迁移等价式与业务合并规则另由业务测试验证。

实现入口：[Transaction.rs](../core/crates/persistence/store/src/Transaction.rs)。

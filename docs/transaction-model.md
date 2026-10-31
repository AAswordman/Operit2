# 事务模型

用 $X\in f$ 表示 X 实现统一接口；$\sqcup$ 表示带类型标记的并集：

$$
f=f_{\mathrm{DataStore}}\sqcup f_{\mathrm{SQL}},\qquad
A,B\in f,\qquad T_a\in\operatorname{Tx}(A)\subseteq f_{\mathrm{SQL}}\subseteq f.
$$

$\operatorname{Tx}(X)$ 是以 X 为目标的事务记录集合。事务的 SQL 载体与其修改目标分别定型；f 对事务暴露目标及操作内容，外层保留身份、修改顺序和原子边界。

| 记号 | 含义 |
| --- | --- |
| $A,B;\ C(A)$ | 两端完整业务状态；$C(A)$ 是确定序列化的文件值或同步 SQL 表结构及记录；重放状态及覆盖信息保存在本机 SQL 账本。 |
| $T_a;\ \mathcal T_A=(T_{a_1},\ldots,T_{a_n})$ | 单个事务；A 端的一组事务，序列可以为空。$\rho(T_a)$ 是业务修改顺序，$\rho(A)$ 是文件最后业务修改的顺序。 |
| $\operatorname{Apply}_\mu(A,\mathcal T)$ | 按序列逐个原子应用事务；$\mu$ 是目标数据所属模块的 merge rule。 |
| $[];\ \Vert;\ \pi$ | 空序列；序列拼接；任意到达顺序的排列。 |
| $\cup_{\mathrm{id}};\ N(\mathcal H)$ | 按身份合并历史；对历史按身份去重并按 $\rho$ 排序。同身份同原文计入一次，同身份不同原文判错。 |
| $\mathsf M_{\mu,b}(\mathcal H)=(b_c,\mathcal H_c)$ | 事务合并：将基线 b 和历史改写为等价基线 $b_c$ 及更小的历史 $\mathcal H_c$。 |
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

$\mu_0,\mu_1$ 为迁移前后的对应规则。此式分别约束 $m_F$、$m_L$ 及其组合。存续事务保持原文；已合并历史按下述覆盖规则回收。转换视图沿用身份、业务修订和原子边界，一个源事务展开的多个操作仍整体提交。需要上下文的规则使用完整基线及身份映射。

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

**事务合并。** 用更小的事务历史或基线代替已被覆盖的修改，保持后续同步的业务结果。记原基线 $b=(A_0,K_b)$、合并后基线 $b_c=(A_c,K_c)$，$K_c$ 为替代表示覆盖的原事务身份；对任意符合 owner 规则及依赖约束的外来历史 $\mathcal U$，须满足：

$$
\mathsf M_{\mu,b}(\mathcal H)=(b_c,\mathcal H_c)
\quad\Longrightarrow\quad
R_b(\mathcal H\cup_{\mathrm{id}}\mathcal U)
\equiv R_{b_c}\!\left(\mathcal H_c\cup_{\mathrm{id}}(\mathcal U\setminus K_c)\right).
$$

$R_b$ 为以上带基线的重放；$\equiv$ 比较业务值、修改范围优先级、删除事实、别名和增量贡献，允许日志表示不同。只比较合并当下的文件值不足以证明此式。

同一归属、同一字段的绝对赋值 $W(k,x,r)$，按后来修改生效。若 $r_1<r_2$，且两个事务仅含该赋值，则：

$$
[W(k,\text{A},r_1),W(k,\text{AB},r_2)]
\xrightarrow{\mathsf M}[W(k,\text{AB},r_2)],\qquad
K_c\setminus K_b=\{\operatorname{id}(W(k,\text{A},r_1))\}.
$$

最终值仍由全部写入的最大业务顺序决定；外来写入排在 $r_2$ 之前时取 AB，排在其后时取外来值。AB 沿用原身份和 $r_2$。`AB` 在这里表示完整赋值，字符串追加和数值增量按各自规则证明等价。

合并以整个原子事务为单位：$T_1=(x:=\text{A},y:=1)$、$T_2=(x:=\text{AB})$ 时，$T_1$ 的 y 仍有效，整条保留；多个后继事务覆盖其全部效果后才可回收。覆盖关系由迁移后规范归属及操作语义判定。SQL 约束、业务投影和身份映射也属于须保留的效果。高阶事务仍需引用原文时保留依赖，或一并折入可重放的替代表示。

两种迁移均须满足合并相容性；记 $Q=(b,\mathcal H)$、$\mathsf M_\mu(Q)=\mathsf M_{\mu,b}(\mathcal H)$；$m(Q)$ 同时转换基线、事务和归属覆盖凭据：

$$
m_F(\mathsf M_{\mu_0}(Q))\equiv\mathsf M_{\mu_1}(m_F(Q)),\qquad
m_L(\mathsf M_{\mu_0}(Q))\equiv\mathsf M_{\mu_1}(m_L(Q)).
$$

工程上，替代基线、剩余事务和覆盖凭据共同持久提交后回收旧 payload；导出先保证对端取得完整替代表示，再确认覆盖身份。离线端通过基线接续，已覆盖历史按空效果确认，其余修改继续按原顺序合并。覆盖按 owner 和来源保存精确区间，空洞、必要指纹及依赖继续保留。未来格式、归属未就绪的待处理事务继续保留。重试保持幂等，合并本身沿用业务修订。

**最终强制校正。** 两端迁移和本轮增量交换完成后，以各自导出范围判断比较是否过期；新事务进入下一轮增量。全量校正的前提为：

$$
(\widehat e_A=e_A)\land(\widehat e_B=e_B)\land C(A')\ne C(B').
$$

该前提允许 $e_A\ne e_B$，无需两端同时采样。历史覆盖信息差异本身不触发全量。$d(X)$ 为固定节点优先序，按业务修订、节点优先序依次比较：

$$
X^*=\underset{X\in\{A',B'\}}{\arg\max}\;(\rho(X),d(X)),\qquad
C(A_{\mathrm{final}})=C(B_{\mathrm{final}})=C(X^*).
$$

直接复制 $X^*$ 的完整同步文件，并在两端保存相同初始化事务，记其状态为 $A_0^*=X^*$。令 $K_A,K_B$ 为各端校正前已处理事务的身份集合，则：

$$
K^*=K_A\cup K_B,\qquad
A_{\mathrm{next}}=B_{\mathrm{next}}=\operatorname{Apply}_{\mu_1}\!\left(A_0^*,N(\mathcal H_{\mathrm{next}}\setminus K^*)\right).
$$

$\setminus K^*$ 按事务身份排除已封存历史。强制校正先确定权威内容，再以事务合并封存该结果：

$$
\mathsf M^*(A_0^*,K^*)=((A_0^*,K^*),[]),\qquad
\operatorname{Restore}(\varnothing,\operatorname{Init}(A_0^*,K^*))\equiv A_0^*.
$$

$\operatorname{Init}$ 是整文件及全部条目的初始化事务；接收端整体发布一次，效果等于从该文件重新开始。初始化事务取得新标识，条目沿用原业务修订、范围版本、删除事实和别名；校正时刻仅记录封存时间。替代表示提交后，该 owner 被 $K^*$ 覆盖且依赖已处理的旧事务 payload 可全部删除。$K^*$ 来自实际覆盖身份，离线端尚未交换的事务按原顺序处理；墙上时钟早于校正时刻不构成覆盖证明。

定义故障因素 $\mathcal F=(\widehat e_A=e_A)\land(\widehat e_B=e_B)\land C(A')\ne C(B')$。故障恢复是人为规定的权威替换：

$$
\operatorname{Correct}_{\mathcal F}(A',\mathcal H_A;B',\mathcal H_B)
=\bigl(X^*,[\operatorname{Init}(X^*,K^*)];X^*,[\operatorname{Init}(X^*,K^*)]\bigr).
$$

这一步允许 $X^*\ne R(\mathcal H_A\cup_{\mathrm{id}}\mathcal H_B)$，以修复后的文件为准；普通合并的重放等价式适用于无故障历史。两端删除 $K^*$ 内的旧载荷，保留收据与可验证的身份指纹；损坏载荷也可依据已提交的身份元数据回收。尚未交换的事务在 $K^*$ 外，继续按原顺序应用。

$\mathsf M^*$ 封存的是已选定的校正结果；普通合并 $\mathsf M$ 保持合并前后的业务语义。后续应用以复制后的文件为起点。缺失文件按同一流程补齐。两端分别按捕获版本原子保存文件及初始化事务，再次比较；期间写入则重新同步、选源和比较。一致后结束本轮同步。记 $\operatorname{Complete}(A,B)$ 为本轮完成，$\operatorname{ready}(A)$ 包含两种迁移、事务处理及对应校验协议均就绪，则：

$$
\operatorname{Complete}(A,B)\Rightarrow\operatorname{ready}(A)\land\operatorname{ready}(B)\land C(A)=C(B).
$$

记 $I(X)$ 为该文件已提交的初始化事务。全量校正后，两端具有同一条初始化事务：

$$
\operatorname{CorrectedComplete}(A,B)\Rightarrow\operatorname{Complete}(A,B)\land I(A)=I(B).
$$

强制校正保证最终内容相等；迁移等价式与业务合并规则另由业务测试验证。

实现入口：[Transaction.rs](../core/crates/persistence/store/src/Transaction.rs)。

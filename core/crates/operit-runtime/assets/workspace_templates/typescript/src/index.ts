// Operit TypeScript 项目
console.log('🚀 欢迎来到 Operit TypeScript 项目！');
console.log('='.repeat(50));
console.log('这是一个 TypeScript 项目模板，您可以：');
console.log('  ✨ 编写类型安全的 TypeScript 代码');
console.log('  📦 使用 pnpm 管理依赖');
console.log('  🔄 使用 tsc watch 实时编译');
console.log('='.repeat(50));

// 接口示例
interface User {
  name: string;
  age: number;
}

// 示例代码
const greeting: string = "Hello from Operit!";
console.log(`\n${greeting}\n`);

// 类型安全的对象
const user: User = {
  name: "Operit User",
  age: 25
};
console.log(`用户信息: ${user.name}, 年龄: ${user.age}`);

// 数组示例
const numbers: number[] = [1, 2, 3, 4, 5];
const sum: number = numbers.reduce((acc, num) => acc + num, 0);
console.log(`数组 [${numbers}] 的总和是: ${sum}`);

console.log('\n✅ 程序运行成功！');
console.log('💡 提示：修改 src/index.ts 后运行 pnpm build 重新编译');

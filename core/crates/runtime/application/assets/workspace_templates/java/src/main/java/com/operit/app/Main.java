package com.operit.app;

/**
 * Operit Java 项目
 * 使用标准的 Gradle 项目结构
 */
public class Main {
    public static void main(String[] args) {
        System.out.println("🚀 欢迎来到 Operit Java 项目！");
        System.out.println("=".repeat(50));
        System.out.println("这是一个标准的 Gradle Java 项目，您可以：");
        System.out.println("  ✨ 编写和编译 Java 代码");
        System.out.println("  📦 使用 Gradle 管理依赖");
        System.out.println("  🏗️ 构建和运行 Java 应用");
        System.out.println("  🧪 编写和运行单元测试");
        System.out.println("=".repeat(50));
        
        // 示例代码
        Calculator calc = new Calculator();
        int result = calc.add(5, 3);
        System.out.println("\n计算示例: 5 + 3 = " + result);
        
        // 数组处理示例
        int[] numbers = {1, 2, 3, 4, 5};
        int sum = calc.sum(numbers);
        System.out.println("数组总和: " + sum + "\n");
        
        System.out.println("✅ 程序运行成功！");
    }
}

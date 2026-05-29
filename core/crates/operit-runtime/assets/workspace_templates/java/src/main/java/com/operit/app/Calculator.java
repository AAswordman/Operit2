package com.operit.app;

/**
 * 简单的计算器类
 * 用于演示类结构和单元测试
 */
public class Calculator {
    
    /**
     * 加法运算
     */
    public int add(int a, int b) {
        return a + b;
    }
    
    /**
     * 减法运算
     */
    public int subtract(int a, int b) {
        return a - b;
    }
    
    /**
     * 乘法运算
     */
    public int multiply(int a, int b) {
        return a * b;
    }
    
    /**
     * 除法运算
     */
    public double divide(int a, int b) {
        if (b == 0) {
            throw new ArithmeticException("除数不能为0");
        }
        return (double) a / b;
    }
    
    /**
     * 计算数组总和
     */
    public int sum(int[] numbers) {
        int total = 0;
        for (int num : numbers) {
            total += num;
        }
        return total;
    }
}

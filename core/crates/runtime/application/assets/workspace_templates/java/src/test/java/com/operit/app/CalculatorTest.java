package com.operit.app;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.DisplayName;
import static org.junit.jupiter.api.Assertions.*;

/**
 * Calculator 类的单元测试
 */
@DisplayName("Calculator Tests")
class CalculatorTest {
    
    private final Calculator calculator = new Calculator();
    
    @Test
    @DisplayName("加法测试")
    void testAdd() {
        assertEquals(8, calculator.add(5, 3));
        assertEquals(0, calculator.add(-5, 5));
        assertEquals(-8, calculator.add(-5, -3));
    }
    
    @Test
    @DisplayName("减法测试")
    void testSubtract() {
        assertEquals(2, calculator.subtract(5, 3));
        assertEquals(-10, calculator.subtract(-5, 5));
    }
    
    @Test
    @DisplayName("乘法测试")
    void testMultiply() {
        assertEquals(15, calculator.multiply(5, 3));
        assertEquals(0, calculator.multiply(5, 0));
        assertEquals(-15, calculator.multiply(-5, 3));
    }
    
    @Test
    @DisplayName("除法测试")
    void testDivide() {
        assertEquals(2.5, calculator.divide(5, 2));
        assertEquals(0.0, calculator.divide(0, 5));
    }
    
    @Test
    @DisplayName("除以零应该抛出异常")
    void testDivideByZero() {
        assertThrows(ArithmeticException.class, () -> {
            calculator.divide(5, 0);
        });
    }
    
    @Test
    @DisplayName("数组求和测试")
    void testSum() {
        int[] numbers = {1, 2, 3, 4, 5};
        assertEquals(15, calculator.sum(numbers));
        
        int[] emptyArray = {};
        assertEquals(0, calculator.sum(emptyArray));
    }
}

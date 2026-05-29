package main

import (
	"fmt"
	"strings"
)

func main() {
	fmt.Println("🚀 欢迎来到 Operit Go 项目！")
	fmt.Println(strings.Repeat("=", 50))
	fmt.Println("这是一个 Go 项目模板，您可以：")
	fmt.Println("  ✨ 编写和编译 Go 代码")
	fmt.Println("  📦 使用 go mod 管理依赖")
	fmt.Println("  ⚡ 利用 Go 的并发特性")
	fmt.Println(strings.Repeat("=", 50))

	// 示例代码
	greeting := "Hello from Operit!"
	fmt.Printf("\n%s\n\n", greeting)
	
	// 简单的计算示例
	numbers := []int{1, 2, 3, 4, 5}
	sum := 0
	for _, num := range numbers {
		sum += num
	}
	fmt.Printf("数组 %v 的总和是: %d\n", numbers, sum)
	
	// 并发示例
	fmt.Println("\n✅ 程序运行成功！")
	fmt.Println("💡 提示：修改 main.go 文件后运行 go run main.go")
}

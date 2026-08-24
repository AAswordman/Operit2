// Operit Node.js 项目
console.log('🚀 欢迎来到 Operit Node.js 项目！');

// 示例：创建一个简单的 HTTP 服务器
const http = require('http');

const hostname = '127.0.0.1';
const port = 3000;

const server = http.createServer((req, res) => {
    res.statusCode = 200;
    res.setHeader('Content-Type', 'text/html; charset=utf-8');
    res.end(`
    <!DOCTYPE html>
    <html>
    <head>
      <meta charset="UTF-8">
      <title>Operit Node.js</title>
      <style>
        body {
          font-family: system-ui, sans-serif;
          max-width: 800px;
          margin: 50px auto;
          padding: 20px;
          text-align: center;
        }
        h1 { color: #68a063; }
      </style>
    </head>
    <body>
      <h1>🟢 Node.js 服务器运行中</h1>
      <p>恭喜！您的 Operit Node.js 项目已成功启动。</p>
      <p>服务器运行在 http://${hostname}:${port}</p>
    </body>
    </html>
  `);
});

server.listen(port, hostname, () => {
    console.log(`✅ 服务器运行在 http://${hostname}:${port}/`);
    console.log('💡 提示：修改 index.js 文件后重启服务器以查看更改');
});

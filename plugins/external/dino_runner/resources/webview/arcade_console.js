(function () {
  var canvas = document.getElementById("gameCanvas");
  var context = canvas && canvas.getContext ? canvas.getContext("2d") : null;
  if (!canvas || !context) {
    return;
  }

  var FPS = 1000 / 60;
  var BASE_GROUND_MARGIN = 23;
  var VIEWPORT = {
    width: 420,
    height: 150,
    groundY: 150 - BASE_GROUND_MARGIN,
  };
  var SPRITE_URL = "/assets/chrome_dino_sprite.png";
  var DIGIT_WIDTH = 10;
  var DIGIT_HEIGHT = 13;
  var DIGIT_ADVANCE = 11;
  var SCORE_COEFFICIENT = 0.025;
  var MAX_SPEED = 13;
  var MIN_SPAWN_DISTANCE = 170;
  var JUMP_CONFIG = {
    gravity: 0.6,
    baseVelocity: -10,
    dropVelocity: -5,
    maxRise: 62,
  };

  var sprite = new Image();
  var spriteReady = false;

  var spriteMap = {
    cloud: { x: 86, y: 2, width: 46, height: 14 },
    bird: { x: 134, y: 2, width: 46, height: 40 },
    cactusSmall: { x: 228, y: 2, width: 17, height: 35 },
    cactusLarge: { x: 332, y: 2, width: 25, height: 50 },
    horizon: { x: 2, y: 54, width: 600, height: 12 },
    text: { x: 655, y: 2, width: 10, height: 13 },
    trex: { x: 848, y: 2, width: 44, height: 47, duckWidth: 59, duckHeight: 25 },
  };

  var dinoCollisionBoxes = {
    running: [
      { x: 22, y: 0, width: 17, height: 16 },
      { x: 1, y: 18, width: 30, height: 9 },
      { x: 10, y: 35, width: 14, height: 8 },
      { x: 1, y: 24, width: 29, height: 5 },
      { x: 5, y: 30, width: 21, height: 4 },
      { x: 9, y: 34, width: 15, height: 4 },
    ],
    ducking: [{ x: 1, y: 18, width: 55, height: 25 }],
  };

  var obstacleTypes = [
    {
      kind: "cactusSmall",
      width: 17,
      height: 35,
      y: 105,
      multipleSpeed: 4,
      minGap: 120,
      minSpeed: 0,
      collisionBoxes: [
        { x: 0, y: 7, width: 5, height: 27 },
        { x: 4, y: 0, width: 6, height: 34 },
        { x: 10, y: 4, width: 7, height: 14 },
      ],
    },
    {
      kind: "cactusLarge",
      width: 25,
      height: 50,
      y: 90,
      multipleSpeed: 7,
      minGap: 120,
      minSpeed: 0,
      collisionBoxes: [
        { x: 0, y: 12, width: 7, height: 38 },
        { x: 8, y: 0, width: 7, height: 49 },
        { x: 13, y: 10, width: 10, height: 38 },
      ],
    },
    {
      kind: "bird",
      width: 46,
      height: 40,
      yOptions: [100, 75, 50],
      minGap: 150,
      minSpeed: 8.5,
      speedOffset: 0.8,
      collisionBoxes: [
        { x: 15, y: 15, width: 16, height: 5 },
        { x: 18, y: 21, width: 24, height: 6 },
        { x: 2, y: 14, width: 4, height: 3 },
        { x: 6, y: 10, width: 4, height: 7 },
        { x: 10, y: 8, width: 6, height: 9 },
      ],
    },
  ];

  var layout = {
    width: window.innerWidth,
    height: window.innerHeight,
    scale: 1,
    offsetX: 0,
    offsetY: 0,
  };

  var state = {
    running: false,
    paused: false,
    crashed: false,
    score: 0,
    bestScore: 0,
    distance: 0,
    speed: 6,
    acceleration: 0.001,
    lastPublishedScore: -1,
    lastPublishedAt: 0,
    introText: "轻触屏幕或按空格开始",
  };

  var dino = {
    x: 50,
    y: VIEWPORT.groundY - spriteMap.trex.height,
    velocityY: 0,
    jumping: false,
    ducking: false,
    speedDrop: false,
    reachedMinHeight: false,
    blinkTimer: 0,
    animTimer: 0,
    currentFrame: 0,
  };

  var world = {
    clouds: [],
    obstacles: [],
    horizonX: [0, VIEWPORT.width],
    horizonSourceX: [spriteMap.horizon.x, spriteMap.horizon.x + spriteMap.horizon.width],
    frameHandle: 0,
    lastTime: 0,
    pointerStartY: 0,
  };

  function getHost() {
    return window.ArcadeHost || null;
  }

  function hostReady() {
    var host = getHost();
    return !!(host && typeof host.updateOverlayScore === "function");
  }

  function publishOverlayScore(force) {
    var host = getHost();
    if (!host || typeof host.updateOverlayScore !== "function") {
      return;
    }
    if (!force && state.lastPublishedScore === state.score) {
      return;
    }
    var now = Date.now();
    if (!force && now - state.lastPublishedAt < 120) {
      return;
    }
    state.lastPublishedScore = state.score;
    state.lastPublishedAt = now;
    host.updateOverlayScore({ score: state.score });
  }

  function resizeCanvas() {
    var ratio = Math.max(1, Math.min(2, window.devicePixelRatio || 1));
    var width = Math.max(
      320,
      Math.round(window.innerWidth || document.documentElement.clientWidth || 360)
    );
    var height = Math.max(
      420,
      Math.round(window.innerHeight || document.documentElement.clientHeight || 640)
    );
    var horizontalInset = Math.max(18, Math.min(34, Math.round(width * 0.08)));
    var playableWidth = Math.max(280, Math.min(460, width - horizontalInset * 2));
    var playableHeight = Math.max(136, Math.min(176, Math.round(height * 0.2)));
    VIEWPORT.width = playableWidth;
    VIEWPORT.height = playableHeight;
    VIEWPORT.groundY = VIEWPORT.height - BASE_GROUND_MARGIN;
    dino.x = Math.max(36, Math.round(VIEWPORT.width * 0.085));
    canvas.width = Math.round(width * ratio);
    canvas.height = Math.round(height * ratio);
    canvas.style.width = width + "px";
    canvas.style.height = height + "px";
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    layout.width = width;
    layout.height = height;
    layout.scale = 1;
    layout.offsetX = Math.round((width - VIEWPORT.width) / 2);
    layout.offsetY = Math.round(
      Math.max(26, Math.min(84, Math.round(height * 0.18)))
    );
  }

  function resetDino() {
    dino.y = VIEWPORT.groundY - spriteMap.trex.height;
    dino.velocityY = 0;
    dino.jumping = false;
    dino.ducking = false;
    dino.speedDrop = false;
    dino.reachedMinHeight = false;
    dino.blinkTimer = 0;
    dino.animTimer = 0;
    dino.currentFrame = 0;
  }

  function resetWorld() {
    world.clouds = [
      { x: Math.round(VIEWPORT.width * 0.58), y: 30 },
      { x: Math.round(VIEWPORT.width * 0.84), y: 48 },
    ];
    world.obstacles = [];
    world.horizonX = [0, VIEWPORT.width];
    world.horizonSourceX = [spriteMap.horizon.x, spriteMap.horizon.x + spriteMap.horizon.width];
  }

  function resetGame(silentBridge) {
    state.score = 0;
    state.distance = 0;
    state.speed = 6;
    state.paused = false;
    state.crashed = false;
    state.lastPublishedScore = -1;
    state.lastPublishedAt = 0;
    resetDino();
    resetWorld();
    if (!silentBridge) {
      publishOverlayScore(true);
    }
  }

  function startGame(silentBridge) {
    resetGame(silentBridge === true);
    state.running = true;
  }

  function setPaused(paused) {
    if (!state.running || state.crashed) {
      return;
    }
    state.paused = !!paused;
  }

  function togglePause() {
    if (!state.running || state.crashed) {
      return;
    }
    state.paused = !state.paused;
  }

  function handlePrimaryAction(options) {
    var silentBridge = !!(options && options.silentBridge);
    if (!state.running || state.crashed) {
      startGame(silentBridge);
      return { ok: true, action: "start", state: snapshot() };
    }
    togglePause();
    return { ok: true, action: state.paused ? "pause" : "resume", state: snapshot() };
  }

  function hostPrimaryAction() {
    return handlePrimaryAction({ silentBridge: true });
  }

  function hostResetBoard() {
    resetGame(true);
    state.running = false;
    return { ok: true, action: "reset", state: snapshot() };
  }

  function startJump() {
    if (!state.running || state.paused || state.crashed || dino.jumping) {
      return;
    }
    dino.jumping = true;
    dino.speedDrop = false;
    dino.velocityY = JUMP_CONFIG.baseVelocity - state.speed / 10;
  }

  function setDuck(isDucking) {
    if (state.crashed) {
      return;
    }
    dino.ducking = !!isDucking && !dino.jumping;
  }

  function speedDrop() {
    if (dino.jumping) {
      dino.speedDrop = true;
      dino.velocityY = Math.max(1, dino.velocityY);
    }
  }

  function handleJumpInput() {
    if (!spriteReady) {
      return;
    }
    if (state.crashed) {
      startGame(false);
      startJump();
      return;
    }
    if (!state.running) {
      startGame(false);
      startJump();
      return;
    }
    if (state.paused) {
      setPaused(false);
      return;
    }
    startJump();
  }

  function createObstacleConfig() {
    var available = [];
    for (var i = 0; i < obstacleTypes.length; i += 1) {
      if (state.speed >= obstacleTypes[i].minSpeed) {
        available.push(obstacleTypes[i]);
      }
    }
    return available[Math.floor(Math.random() * available.length)];
  }

  function cloneCollisionBoxes(source) {
    var boxes = [];
    for (var i = 0; i < source.length; i += 1) {
      boxes.push({
        x: source[i].x,
        y: source[i].y,
        width: source[i].width,
        height: source[i].height,
      });
    }
    return boxes;
  }

  function getGap(width, minGap, speed) {
    var base = Math.round(width * speed + minGap * 1.5);
    return randomInt(base, Math.round(base * 1.5));
  }

  function spawnObstacle() {
    var type = createObstacleConfig();
    if (!type) {
      return;
    }
    var size = 1;
    if (type.kind !== "bird" && state.speed >= type.multipleSpeed) {
      size = randomInt(1, 3);
    }
    var width = type.width * size;
    var y = Array.isArray(type.yOptions)
      ? type.yOptions[randomInt(0, type.yOptions.length - 1)]
      : type.y;
    var obstacle = {
      kind: type.kind,
      x: VIEWPORT.width + 20,
      y: y,
      width: width,
      height: type.height,
      size: size,
      frame: 0,
      frameTimer: 0,
      speedOffset: type.speedOffset || 0,
      collisionBoxes: cloneCollisionBoxes(type.collisionBoxes),
      gap: getGap(width, type.minGap, state.speed),
      spriteX:
        (type.width * size) * (0.5 * (size - 1)) +
        spriteMap[type.kind === "bird" ? "bird" : type.kind].x,
    };
    if (size > 1 && obstacle.collisionBoxes.length >= 3) {
      obstacle.collisionBoxes[1].width =
        width - obstacle.collisionBoxes[0].width - obstacle.collisionBoxes[2].width;
      obstacle.collisionBoxes[2].x = width - obstacle.collisionBoxes[2].width;
    }
    world.obstacles.push(obstacle);
  }

  function maybeSpawnObstacle() {
    if (!state.running || state.paused || state.crashed) {
      return;
    }
    if (!world.obstacles.length) {
      spawnObstacle();
      return;
    }
    var last = world.obstacles[world.obstacles.length - 1];
    if (last.x + last.width + last.gap < VIEWPORT.width) {
      spawnObstacle();
    }
  }

  function updateClouds(frameScale) {
    if (!world.clouds.length) {
      world.clouds.push({ x: VIEWPORT.width, y: randomInt(18, 56) });
    }
    for (var i = world.clouds.length - 1; i >= 0; i -= 1) {
      world.clouds[i].x -= 0.5 * frameScale;
      if (world.clouds[i].x + spriteMap.cloud.width < -12) {
        world.clouds.splice(i, 1);
      }
    }
    var last = world.clouds[world.clouds.length - 1];
    if (!last || VIEWPORT.width - last.x > randomInt(160, 280)) {
      world.clouds.push({
        x: VIEWPORT.width + randomInt(12, 60),
        y: randomInt(18, 56),
      });
    }
  }

  function updateHorizon(frameScale) {
    var increment = Math.floor(state.speed * frameScale);
    for (var i = 0; i < 2; i += 1) {
      world.horizonX[i] -= increment;
    }
    if (world.horizonX[0] <= -VIEWPORT.width) {
      world.horizonX[0] += VIEWPORT.width * 2;
      world.horizonSourceX[0] =
        spriteMap.horizon.x + (Math.random() > 0.5 ? 0 : spriteMap.horizon.width);
    }
    if (world.horizonX[1] <= -VIEWPORT.width) {
      world.horizonX[1] += VIEWPORT.width * 2;
      world.horizonSourceX[1] =
        spriteMap.horizon.x + (Math.random() > 0.5 ? 0 : spriteMap.horizon.width);
    }
  }

  function updateDino(deltaMs) {
    if (!dino.jumping) {
      return;
    }
    var framesElapsed = deltaMs / FPS;
    var speedDropCoefficient = 3;
    var groundDinoY = VIEWPORT.groundY - spriteMap.trex.height;
    var maxJumpY = groundDinoY - JUMP_CONFIG.maxRise;
    if (dino.speedDrop) {
      dino.y += Math.round(dino.velocityY * speedDropCoefficient * framesElapsed);
    } else {
      dino.y += Math.round(dino.velocityY * framesElapsed);
    }
    dino.velocityY += JUMP_CONFIG.gravity * framesElapsed;
    if ((dino.y < maxJumpY || dino.speedDrop) && dino.velocityY < JUMP_CONFIG.dropVelocity) {
      dino.velocityY = JUMP_CONFIG.dropVelocity;
    }
    if (dino.y >= groundDinoY) {
      resetDino();
    }
  }

  function getDinoCollisionBoxes() {
    return dino.ducking ? dinoCollisionBoxes.ducking : dinoCollisionBoxes.running;
  }

  function collisionDetected(obstacle) {
    var dinoBoxes = getDinoCollisionBoxes();
    for (var i = 0; i < dinoBoxes.length; i += 1) {
      var dinoBox = dinoBoxes[i];
      var a = {
        x: dino.x + dinoBox.x,
        y: dino.y + dinoBox.y,
        width: dinoBox.width,
        height: dinoBox.height,
      };
      for (var j = 0; j < obstacle.collisionBoxes.length; j += 1) {
        var obsBox = obstacle.collisionBoxes[j];
        var b = {
          x: obstacle.x + obsBox.x,
          y: obstacle.y + obsBox.y,
          width: obsBox.width,
          height: obsBox.height,
        };
        if (
          a.x < b.x + b.width &&
          a.x + a.width > b.x &&
          a.y < b.y + b.height &&
          a.y + a.height > b.y
        ) {
          return true;
        }
      }
    }
    return false;
  }

  function crash() {
    state.running = false;
    state.paused = false;
    state.crashed = true;
    state.bestScore = Math.max(state.bestScore, state.score);
    dino.ducking = false;
    dino.jumping = false;
    dino.velocityY = 0;
    publishOverlayScore(true);
  }

  function updateObstacles(deltaMs, frameScale) {
    maybeSpawnObstacle();
    for (var i = world.obstacles.length - 1; i >= 0; i -= 1) {
      var obstacle = world.obstacles[i];
      var travelSpeed = state.speed + obstacle.speedOffset;
      obstacle.x -= Math.floor(travelSpeed * frameScale);
      if (obstacle.kind === "bird") {
        obstacle.frameTimer += deltaMs;
        if (obstacle.frameTimer >= 1000 / 6) {
          obstacle.frameTimer = 0;
          obstacle.frame = obstacle.frame === 0 ? 1 : 0;
        }
      }
      if (collisionDetected(obstacle)) {
        crash();
        return;
      }
      if (obstacle.x + obstacle.width < -MIN_SPAWN_DISTANCE) {
        world.obstacles.splice(i, 1);
      }
    }
  }

  function updateScore() {
    var nextScore = Math.max(0, Math.round(state.distance * SCORE_COEFFICIENT));
    if (nextScore !== state.score) {
      state.score = nextScore;
      publishOverlayScore(false);
    }
  }

  function update(deltaMs) {
    if (!spriteReady) {
      return;
    }
    if (state.running && !state.paused && !state.crashed) {
      var frameScale = deltaMs / FPS;
      state.speed = Math.min(MAX_SPEED, state.speed + state.acceleration * deltaMs);
      state.distance += state.speed * frameScale;
      updateScore();
      updateClouds(frameScale);
      updateHorizon(frameScale);
      updateDino(deltaMs);
      updateObstacles(deltaMs, frameScale);
      dino.animTimer += deltaMs;
      if (dino.animTimer >= 1000 / 12) {
        dino.animTimer = 0;
        dino.currentFrame = dino.currentFrame === 0 ? 1 : 0;
      }
      return;
    }

    if (state.crashed) {
      dino.currentFrame = 2;
      return;
    }

    dino.animTimer += deltaMs;
    if (dino.animTimer >= 1000 / 3) {
      dino.animTimer = 0;
      dino.currentFrame = dino.currentFrame === 3 ? 0 : 3;
    }
  }

  function drawSprite(sourceX, sourceY, sourceWidth, sourceHeight, x, y, width, height) {
    context.drawImage(sprite, sourceX, sourceY, sourceWidth, sourceHeight, x, y, width, height);
  }

  function drawBackground() {
    context.fillStyle = "#f7f7f7";
    context.fillRect(0, 0, layout.width, layout.height);
    context.save();
    context.translate(layout.offsetX, layout.offsetY);
    context.scale(layout.scale, layout.scale);
    context.fillStyle = "#f7f7f7";
    context.fillRect(0, 0, VIEWPORT.width, VIEWPORT.height);
    context.restore();
  }

  function drawClouds() {
    for (var i = 0; i < world.clouds.length; i += 1) {
      drawSprite(
        spriteMap.cloud.x,
        spriteMap.cloud.y,
        spriteMap.cloud.width,
        spriteMap.cloud.height,
        world.clouds[i].x,
        world.clouds[i].y,
        spriteMap.cloud.width,
        spriteMap.cloud.height
      );
    }
  }

  function drawHorizon() {
    for (var i = 0; i < 2; i += 1) {
      drawSprite(
        world.horizonSourceX[i],
        spriteMap.horizon.y,
        spriteMap.horizon.width,
        spriteMap.horizon.height,
        world.horizonX[i],
        VIEWPORT.groundY,
        VIEWPORT.width,
        spriteMap.horizon.height
      );
    }
  }

  function drawObstacle(obstacle) {
    if (obstacle.kind === "bird") {
      drawSprite(
        spriteMap.bird.x + spriteMap.bird.width * obstacle.frame,
        spriteMap.bird.y,
        spriteMap.bird.width,
        spriteMap.bird.height,
        obstacle.x,
        obstacle.y,
        spriteMap.bird.width,
        spriteMap.bird.height
      );
      return;
    }
    drawSprite(
      obstacle.spriteX,
      spriteMap[obstacle.kind].y,
      spriteMap[obstacle.kind].width * obstacle.size,
      spriteMap[obstacle.kind].height,
      obstacle.x,
      obstacle.y,
      obstacle.width,
      obstacle.height
    );
  }

  function drawDino() {
    var frameOffset = 0;
    var width = spriteMap.trex.width;
    var height = spriteMap.trex.height;
    if (state.crashed) {
      frameOffset = 220;
    } else if (dino.jumping) {
      frameOffset = 0;
    } else if (dino.ducking) {
      width = spriteMap.trex.duckWidth;
      height = spriteMap.trex.duckHeight;
      frameOffset = dino.currentFrame === 0 ? 264 : 323;
    } else if (!state.running) {
      frameOffset = dino.currentFrame === 0 ? 44 : 0;
    } else {
      frameOffset = dino.currentFrame === 0 ? 88 : 132;
    }
    drawSprite(
      spriteMap.trex.x + frameOffset,
      spriteMap.trex.y,
      width,
      height,
      dino.x,
      dino.ducking ? VIEWPORT.groundY - height : dino.y,
      width,
      height
    );
  }

  function drawScoreDigits(value, x, y) {
    var digits = ("00000" + Math.max(0, value)).slice(-5).split("");
    for (var i = 0; i < digits.length; i += 1) {
      drawSprite(
        spriteMap.text.x + DIGIT_WIDTH * Number(digits[i]),
        spriteMap.text.y,
        DIGIT_WIDTH,
        DIGIT_HEIGHT,
        x + i * DIGIT_ADVANCE,
        y,
        DIGIT_WIDTH,
        DIGIT_HEIGHT
      );
    }
  }

  function drawHiScore(x, y) {
    drawSprite(
      spriteMap.text.x + DIGIT_WIDTH * 10,
      spriteMap.text.y,
      DIGIT_WIDTH,
      DIGIT_HEIGHT,
      x,
      y,
      DIGIT_WIDTH,
      DIGIT_HEIGHT
    );
    drawSprite(
      spriteMap.text.x + DIGIT_WIDTH * 11,
      spriteMap.text.y,
      DIGIT_WIDTH,
      DIGIT_HEIGHT,
      x + DIGIT_ADVANCE,
      y,
      DIGIT_WIDTH,
      DIGIT_HEIGHT
    );
    drawScoreDigits(state.bestScore, x + DIGIT_ADVANCE * 2, y);
  }

  function drawPrompt() {
    context.fillStyle = "#535353";
    context.textAlign = "center";
    context.font = "14px 'Microsoft YaHei', sans-serif";
    if (state.crashed) {
      context.fillText("撞上了，轻触或空格重新开始", VIEWPORT.width / 2, 66);
      context.fillText("GAME OVER", VIEWPORT.width / 2, 88);
    } else if (state.paused) {
      context.fillText("已暂停，点击开始/暂停或轻触继续", VIEWPORT.width / 2, 74);
    } else if (!state.running) {
      context.fillText(state.introText, VIEWPORT.width / 2, 74);
    }
    context.textAlign = "left";
  }

  function drawScene() {
    drawBackground();
    if (!spriteReady) {
      context.fillStyle = "#535353";
      context.textAlign = "center";
      context.font = "16px 'Microsoft YaHei', sans-serif";
      context.fillText("正在装载小恐龙资源...", layout.width / 2, layout.height / 2);
      context.textAlign = "left";
      return;
    }

    context.save();
    context.translate(layout.offsetX, layout.offsetY);
    context.scale(layout.scale, layout.scale);

    drawClouds();
    drawHorizon();
    for (var i = 0; i < world.obstacles.length; i += 1) {
      drawObstacle(world.obstacles[i]);
    }
    drawDino();
    drawScoreDigits(state.score, VIEWPORT.width - 63, 5);
    drawHiScore(VIEWPORT.width - 140, 5);
    drawPrompt();

    context.restore();
  }

  function snapshot() {
    return {
      score: state.score,
      bestScore: state.bestScore,
      runActive: state.running,
      runPaused: state.paused,
      crashed: state.crashed,
      obstacleCount: world.obstacles.length,
      speed: Number(state.speed.toFixed(2)),
      hostReady: hostReady(),
    };
  }

  function frame(now) {
    if (!world.lastTime) {
      world.lastTime = now;
    }
    var deltaMs = Math.min(40, now - world.lastTime);
    world.lastTime = now;
    update(deltaMs);
    drawScene();
    world.frameHandle = window.requestAnimationFrame(frame);
  }

  function randomInt(min, max) {
    return Math.floor(Math.random() * (max - min + 1)) + min;
  }

  function bindEvents() {
    window.addEventListener("resize", resizeCanvas);
    window.addEventListener("keydown", function (event) {
      if (event.code === "Space" || event.code === "ArrowUp") {
        event.preventDefault();
        handleJumpInput();
        return;
      }
      if (event.code === "ArrowDown") {
        event.preventDefault();
        if (dino.jumping) {
          speedDrop();
          return;
        }
        setDuck(true);
        return;
      }
      if (event.code === "KeyP") {
        event.preventDefault();
        togglePause();
      }
    });
    window.addEventListener("keyup", function (event) {
      if (event.code === "ArrowDown") {
        setDuck(false);
      }
    });
    canvas.addEventListener("pointerdown", function (event) {
      world.pointerStartY = event.clientY;
      handleJumpInput();
    });
    canvas.addEventListener("pointerup", function (event) {
      if (event.clientY - world.pointerStartY > 24) {
        speedDrop();
      }
    });
  }

  resizeCanvas();
  resetGame(false);
  bindEvents();

  sprite.onload = function () {
    spriteReady = true;
    drawScene();
  };
  sprite.src = SPRITE_URL;

  window.__arcadeConsole = {
    getSnapshot: snapshot,
    hostPrimaryAction: hostPrimaryAction,
    hostResetBoard: hostResetBoard,
    togglePause: togglePause,
    resetBoard: function () {
      resetGame(false);
      state.running = false;
      return snapshot();
    },
    startRun: function () {
      startGame(false);
      return snapshot();
    },
  };

  world.frameHandle = window.requestAnimationFrame(frame);
})();

(function () {
  var canvas = document.getElementById("gameCanvas");
  var context = canvas && canvas.getContext ? canvas.getContext("2d") : null;
  if (!canvas || !context) {
    return;
  }

  var FPS = 60;
  var FRAME_TIME = 1000 / FPS;
  var SPRITE_URL = "/assets/chrome_dino_sprite.png";
  var BG_COLOR = "#f7f7f7";

  var DEFAULT_DIMENSIONS = {
    width: 600,
    height: 150,
  };

  var CONFIG = {
    acceleration: 0.001,
    bgCloudSpeed: 0.2,
    bottomPad: 10,
    clearTime: 3000,
    cloudFrequency: 0.5,
    gameoverClearTime: 1200,
    gapCoefficient: 0.6,
    maxBlinkCount: 3,
    maxClouds: 6,
    maxObstacleDuplication: 2,
    maxObstacleLength: 3,
    maxSpeed: 13,
    mobileSpeedCoefficient: 1.2,
    speed: 6,
    speedDropCoefficient: 3,
    arcadeModeInitialTopPosition: 35,
    arcadeModeTopPositionPercent: 0.1,
  };

  var TREX_CONFIG = {
    dropVelocity: -5,
    gravity: 0.6,
    height: 47,
    initialJumpVelocity: -10,
    maxJumpHeight: 30,
    minJumpHeight: 30,
    speedDropCoefficient: 3,
    startXPos: 50,
    width: 44,
    widthDuck: 59,
  };

  var sprite = new Image();
  var spriteReady = false;

  var spriteMap = {
    cloud: { x: 86, y: 2, width: 46, height: 14 },
    cactusLarge: { x: 332, y: 2, width: 25, height: 50 },
    cactusSmall: { x: 228, y: 2, width: 17, height: 35 },
    horizon: { x: 2, y: 52, width: 600, height: 12, yPos: 127 },
    pterodactyl: { x: 134, y: 2, width: 46, height: 40 },
    restart: { x: 2, y: 68, width: 36, height: 32 },
    text: { x: 655, y: 2, width: 10, height: 13, advance: 11 },
    trex: { x: 848, y: 2, width: 44, height: 47, widthDuck: 59 },
  };

  var trexCollisionBoxes = {
    ducking: [{ x: 1, y: 18, width: 55, height: 25 }],
    running: [
      { x: 22, y: 0, width: 17, height: 16 },
      { x: 1, y: 18, width: 30, height: 9 },
      { x: 10, y: 35, width: 14, height: 8 },
      { x: 1, y: 24, width: 29, height: 5 },
      { x: 5, y: 30, width: 21, height: 4 },
      { x: 9, y: 34, width: 15, height: 4 },
    ],
  };

  var obstacleTypes = [
    {
      type: "cactusSmall",
      width: 17,
      height: 35,
      yPos: 105,
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
      type: "cactusLarge",
      width: 25,
      height: 50,
      yPos: 90,
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
      type: "pterodactyl",
      width: 46,
      height: 40,
      yPos: [100, 75, 50],
      multipleSpeed: 999,
      minGap: 150,
      minSpeed: 8.5,
      speedOffset: 0.8,
      numFrames: 2,
      frameRate: 1000 / 6,
      collisionBoxes: [
        { x: 15, y: 15, width: 16, height: 5 },
        { x: 18, y: 21, width: 24, height: 6 },
        { x: 2, y: 14, width: 4, height: 3 },
        { x: 6, y: 10, width: 4, height: 7 },
        { x: 10, y: 8, width: 6, height: 9 },
      ],
    },
  ];

  var Status = {
    CRASHED: "crashed",
    DUCKING: "ducking",
    JUMPING: "jumping",
    RUNNING: "running",
    WAITING: "waiting",
  };

  var animFrames = {};
  animFrames[Status.WAITING] = { frames: [44, 0], msPerFrame: 1000 / 3 };
  animFrames[Status.RUNNING] = { frames: [88, 132], msPerFrame: 1000 / 12 };
  animFrames[Status.CRASHED] = { frames: [220], msPerFrame: 1000 / 60 };
  animFrames[Status.JUMPING] = { frames: [0], msPerFrame: 1000 / 60 };
  animFrames[Status.DUCKING] = { frames: [264, 323], msPerFrame: 1000 / 8 };

  var dimensions = {
    width: DEFAULT_DIMENSIONS.width,
    height: DEFAULT_DIMENSIONS.height,
  };

  var layout = {
    width: window.innerWidth || DEFAULT_DIMENSIONS.width,
    height: window.innerHeight || DEFAULT_DIMENSIONS.height,
    scale: 1,
    offsetX: 0,
    offsetY: 0,
    ratio: 1,
  };

  var state = {
    activated: false,
    playing: false,
    paused: false,
    crashed: false,
    score: 0,
    highScore: 0,
    distanceRan: 0,
    runningTime: 0,
    currentSpeed: CONFIG.speed,
    lastPublishedScore: -1,
    lastPublishedAt: 0,
    lastAchievement: 0,
    achievement: false,
    achievementTimer: 0,
    achievementFlashCount: 0,
    gameOverAt: 0,
  };

  var trex = {
    xPos: TREX_CONFIG.startXPos,
    yPos: getTrexGroundY(),
    jumpVelocity: 0,
    jumping: false,
    ducking: false,
    speedDrop: false,
    reachedMinHeight: false,
    jumpCount: 0,
    status: Status.WAITING,
    currentFrame: 0,
    currentAnimFrames: animFrames[Status.WAITING].frames,
    msPerFrame: animFrames[Status.WAITING].msPerFrame,
    timer: 0,
    blinkTimer: 0,
    blinkDelay: 0,
    blinkCount: 0,
  };

  var world = {
    clouds: [],
    obstacles: [],
    obstacleHistory: [],
    horizonX: [0, spriteMap.horizon.width],
    horizonSourceX: [spriteMap.horizon.x, spriteMap.horizon.x + spriteMap.horizon.width],
    frameHandle: 0,
    lastTime: 0,
    pointerActive: false,
  };

  function getHost() {
    return window.ArcadeHost || null;
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

  function hostReady() {
    var host = getHost();
    return !!(host && typeof host.updateOverlayScore === "function");
  }

  function getTrexGroundY() {
    return DEFAULT_DIMENSIONS.height - TREX_CONFIG.height - CONFIG.bottomPad;
  }

  function getTimeStamp() {
    return performance && typeof performance.now === "function"
      ? performance.now()
      : Date.now();
  }

  function randomInt(min, max) {
    return Math.floor(Math.random() * (max - min + 1)) + min;
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

  function setSpeed(newSpeed) {
    var speed = newSpeed || state.currentSpeed;
    if (dimensions.width < DEFAULT_DIMENSIONS.width) {
      var mobileSpeed =
        (speed * dimensions.width * CONFIG.mobileSpeedCoefficient) /
        DEFAULT_DIMENSIONS.width;
      state.currentSpeed = mobileSpeed > speed ? speed : mobileSpeed;
    } else if (newSpeed) {
      state.currentSpeed = newSpeed;
    }
  }

  function adjustDimensions() {
    var width = Math.max(
      240,
      Math.round(window.innerWidth || document.documentElement.clientWidth || 360)
    );
    var height = Math.max(
      240,
      Math.round(window.innerHeight || document.documentElement.clientHeight || 360)
    );
    var horizontalPadding = Math.min(24, Math.max(0, Math.round(width * 0.04)));
    dimensions.width = Math.min(
      DEFAULT_DIMENSIONS.width,
      Math.max(320, width - horizontalPadding * 2)
    );
    dimensions.height = DEFAULT_DIMENSIONS.height;

    var scaleWidth = width / dimensions.width;
    var scaleHeight = height / dimensions.height;
    var scale = Math.min(scaleWidth, scaleHeight);
    if (scale > 1) {
      scale = Math.max(1, Math.min(scaleWidth, scaleHeight));
    }

    var scaledHeight = dimensions.height * scale;
    var translateY = Math.ceil(
      Math.max(0, (height - scaledHeight - CONFIG.arcadeModeInitialTopPosition) *
        CONFIG.arcadeModeTopPositionPercent)
    );

    layout.width = width;
    layout.height = height;
    layout.scale = scale;
    layout.offsetX = Math.round((width - dimensions.width * scale) / 2);
    layout.offsetY = translateY;
    layout.ratio = Math.max(1, Math.min(2, window.devicePixelRatio || 1));

    canvas.width = Math.round(width * layout.ratio);
    canvas.height = Math.round(height * layout.ratio);
    canvas.style.width = width + "px";
    canvas.style.height = height + "px";
    context.setTransform(layout.ratio, 0, 0, layout.ratio, 0, 0);
    context.imageSmoothingEnabled = false;
    setSpeed(state.playing ? state.currentSpeed : CONFIG.speed);
  }

  function setTrexStatus(status) {
    trex.status = status;
    trex.currentFrame = 0;
    trex.timer = 0;
    trex.currentAnimFrames = animFrames[status].frames;
    trex.msPerFrame = animFrames[status].msPerFrame;
    if (status === Status.WAITING) {
      trex.blinkTimer = 0;
      trex.blinkDelay = randomInt(1000, 7000);
    }
  }

  function resetTrex(waiting) {
    trex.xPos = TREX_CONFIG.startXPos;
    trex.yPos = getTrexGroundY();
    trex.jumpVelocity = 0;
    trex.jumping = false;
    trex.ducking = false;
    trex.speedDrop = false;
    trex.reachedMinHeight = false;
    trex.jumpCount = 0;
    trex.blinkCount = 0;
    setTrexStatus(waiting ? Status.WAITING : Status.RUNNING);
  }

  function resetWorld() {
    world.clouds = [];
    world.obstacles = [];
    world.obstacleHistory = [];
    world.horizonX = [0, spriteMap.horizon.width];
    world.horizonSourceX = [spriteMap.horizon.x, spriteMap.horizon.x + spriteMap.horizon.width];
    addCloud();
  }

  function resetGame(silentBridge) {
    state.activated = false;
    state.playing = false;
    state.paused = false;
    state.crashed = false;
    state.score = 0;
    state.distanceRan = 0;
    state.runningTime = 0;
    state.lastAchievement = 0;
    state.achievement = false;
    state.achievementTimer = 0;
    state.achievementFlashCount = 0;
    state.gameOverAt = 0;
    setSpeed(CONFIG.speed);
    resetTrex(true);
    resetWorld();
    if (!silentBridge) {
      publishOverlayScore(true);
    }
  }

  function startRun(silentBridge) {
    state.activated = true;
    state.playing = true;
    state.paused = false;
    state.crashed = false;
    state.score = 0;
    state.distanceRan = 0;
    state.runningTime = 0;
    state.lastAchievement = 0;
    state.achievement = false;
    state.achievementTimer = 0;
    state.achievementFlashCount = 0;
    setSpeed(CONFIG.speed);
    resetTrex(false);
    resetWorld();
    world.lastTime = getTimeStamp();
    if (!silentBridge) {
      publishOverlayScore(true);
    }
  }

  function setPaused(paused) {
    if (!state.playing || state.crashed) {
      return;
    }
    state.paused = !!paused;
    if (!state.paused) {
      world.lastTime = getTimeStamp();
    }
  }

  function togglePause() {
    if (!state.playing || state.crashed) {
      return;
    }
    setPaused(!state.paused);
  }

  function handlePrimaryAction(options) {
    var silentBridge = !!(options && options.silentBridge);
    if (!state.playing || state.crashed) {
      startRun(silentBridge);
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
    return { ok: true, action: "reset", state: snapshot() };
  }

  function startJump() {
    if (!state.playing || state.paused || state.crashed || trex.jumping || trex.ducking) {
      return;
    }
    setTrexStatus(Status.JUMPING);
    trex.jumpVelocity = TREX_CONFIG.initialJumpVelocity - state.currentSpeed / 10;
    trex.jumping = true;
    trex.reachedMinHeight = false;
    trex.speedDrop = false;
  }

  function endJump() {
    if (trex.reachedMinHeight && trex.jumpVelocity < TREX_CONFIG.dropVelocity) {
      trex.jumpVelocity = TREX_CONFIG.dropVelocity;
    }
  }

  function setDuck(isDucking) {
    if (!state.playing || state.paused || state.crashed) {
      return;
    }
    if (isDucking && trex.status !== Status.DUCKING && !trex.jumping) {
      trex.ducking = true;
      setTrexStatus(Status.DUCKING);
    } else if (!isDucking && trex.status === Status.DUCKING) {
      trex.ducking = false;
      setTrexStatus(Status.RUNNING);
    }
  }

  function setSpeedDrop() {
    if (trex.jumping) {
      trex.speedDrop = true;
      trex.jumpVelocity = 1;
    }
  }

  function handleJumpInput() {
    if (!spriteReady) {
      return;
    }
    if (state.crashed) {
      if (getTimeStamp() - state.gameOverAt < CONFIG.gameoverClearTime) {
        return;
      }
      startRun(false);
      startJump();
      return;
    }
    if (!state.playing) {
      startRun(false);
      startJump();
      return;
    }
    if (state.paused) {
      resetTrex(false);
      setPaused(false);
      return;
    }
    startJump();
  }

  function addCloud() {
    world.clouds.push({
      xPos: dimensions.width,
      yPos: randomInt(30, 71),
      gap: randomInt(100, 400),
      remove: false,
    });
  }

  function updateClouds(deltaTime, speed) {
    var cloudSpeed = (CONFIG.bgCloudSpeed / 1000) * deltaTime * speed;
    if (!world.clouds.length) {
      addCloud();
    }
    for (var i = world.clouds.length - 1; i >= 0; i -= 1) {
      var cloud = world.clouds[i];
      cloud.xPos -= Math.ceil(cloudSpeed);
      if (cloud.xPos + spriteMap.cloud.width <= 0) {
        world.clouds.splice(i, 1);
      }
    }
    var lastCloud = world.clouds[world.clouds.length - 1];
    if (
      world.clouds.length < CONFIG.maxClouds &&
      lastCloud &&
      dimensions.width - lastCloud.xPos > lastCloud.gap &&
      CONFIG.cloudFrequency > Math.random()
    ) {
      addCloud();
    }
  }

  function updateHorizonLine(deltaTime, speed) {
    var increment = Math.floor(speed * (FPS / 1000) * deltaTime);
    var line1 = world.horizonX[0] <= 0 ? 0 : 1;
    var line2 = line1 === 0 ? 1 : 0;
    world.horizonX[line1] -= increment;
    world.horizonX[line2] = world.horizonX[line1] + spriteMap.horizon.width;

    if (world.horizonX[line1] <= -spriteMap.horizon.width) {
      world.horizonX[line1] += spriteMap.horizon.width * 2;
      world.horizonX[line2] = world.horizonX[line1] - spriteMap.horizon.width;
      world.horizonSourceX[line1] =
        spriteMap.horizon.x + (Math.random() > 0.5 ? spriteMap.horizon.width : 0);
    }
  }

  function duplicateObstacleCheck(nextObstacleType) {
    var duplicateCount = 0;
    for (var i = 0; i < world.obstacleHistory.length; i += 1) {
      duplicateCount =
        world.obstacleHistory[i] === nextObstacleType ? duplicateCount + 1 : 0;
    }
    return duplicateCount >= CONFIG.maxObstacleDuplication;
  }

  function getObstacleType(currentSpeed) {
    var obstacleType = obstacleTypes[randomInt(0, obstacleTypes.length - 1)];
    if (duplicateObstacleCheck(obstacleType.type) || currentSpeed < obstacleType.minSpeed) {
      return getObstacleType(currentSpeed);
    }
    return obstacleType;
  }

  function getObstacleGap(width, minGap, speed) {
    var min = Math.round(width * speed + minGap * CONFIG.gapCoefficient);
    var max = Math.round(min * 1.5);
    return randomInt(min, max);
  }

  function createObstacle(type, currentSpeed, xOffset) {
    var size = randomInt(1, CONFIG.maxObstacleLength);
    if (size > 1 && type.multipleSpeed > currentSpeed) {
      size = 1;
    }
    var yPos = Array.isArray(type.yPos)
      ? type.yPos[randomInt(0, type.yPos.length - 1)]
      : type.yPos;
    var width = type.width * size;
    var speedOffset = 0;
    if (type.speedOffset) {
      speedOffset = Math.random() > 0.5 ? type.speedOffset : -type.speedOffset;
    }
    var collisionBoxes = cloneCollisionBoxes(type.collisionBoxes);
    if (size > 1 && collisionBoxes.length >= 3) {
      collisionBoxes[1].width =
        width - collisionBoxes[0].width - collisionBoxes[2].width;
      collisionBoxes[2].x = width - collisionBoxes[2].width;
    }
    return {
      type: type.type,
      xPos: dimensions.width + xOffset,
      yPos: yPos,
      width: width,
      height: type.height,
      size: size,
      gap: getObstacleGap(width, type.minGap, currentSpeed),
      speedOffset: speedOffset,
      collisionBoxes: collisionBoxes,
      frame: 0,
      frameTimer: 0,
      followingObstacleCreated: false,
      remove: false,
    };
  }

  function addNewObstacle(currentSpeed) {
    var obstacleType = getObstacleType(currentSpeed);
    var obstacle = createObstacle(obstacleType, currentSpeed, obstacleType.width);
    world.obstacles.push(obstacle);
    world.obstacleHistory.unshift(obstacleType.type);
    if (world.obstacleHistory.length > CONFIG.maxObstacleDuplication) {
      world.obstacleHistory.splice(CONFIG.maxObstacleDuplication);
    }
  }

  function updateObstacles(deltaTime, currentSpeed) {
    for (var i = world.obstacles.length - 1; i >= 0; i -= 1) {
      var obstacle = world.obstacles[i];
      var speed = currentSpeed + obstacle.speedOffset;
      obstacle.xPos -= Math.floor(speed * (FPS / 1000) * deltaTime);
      if (obstacle.type === "pterodactyl") {
        obstacle.frameTimer += deltaTime;
        if (obstacle.frameTimer >= 1000 / 6) {
          obstacle.frameTimer = 0;
          obstacle.frame = obstacle.frame === 0 ? 1 : 0;
        }
      }
      if (obstacle.xPos + obstacle.width <= 0) {
        world.obstacles.splice(i, 1);
      }
    }

    if (world.obstacles.length > 0) {
      var lastObstacle = world.obstacles[world.obstacles.length - 1];
      if (
        lastObstacle &&
        !lastObstacle.followingObstacleCreated &&
        lastObstacle.xPos + lastObstacle.width > 0 &&
        lastObstacle.xPos + lastObstacle.width + lastObstacle.gap < dimensions.width
      ) {
        addNewObstacle(currentSpeed);
        lastObstacle.followingObstacleCreated = true;
      }
    } else {
      addNewObstacle(currentSpeed);
    }
  }

  function updateTrexJump(deltaTime) {
    var framesElapsed = deltaTime / animFrames[Status.JUMPING].msPerFrame;
    if (trex.speedDrop) {
      trex.yPos += Math.round(
        trex.jumpVelocity * TREX_CONFIG.speedDropCoefficient * framesElapsed
      );
    } else {
      trex.yPos += Math.round(trex.jumpVelocity * framesElapsed);
    }
    trex.jumpVelocity += TREX_CONFIG.gravity * framesElapsed;

    var minJumpY = getTrexGroundY() - TREX_CONFIG.minJumpHeight;
    if (trex.yPos < minJumpY || trex.speedDrop) {
      trex.reachedMinHeight = true;
    }
    if (trex.yPos < TREX_CONFIG.maxJumpHeight || trex.speedDrop) {
      endJump();
    }
    if (trex.yPos > getTrexGroundY()) {
      trex.yPos = getTrexGroundY();
      trex.jumpVelocity = 0;
      trex.jumping = false;
      trex.speedDrop = false;
      trex.reachedMinHeight = false;
      trex.jumpCount += 1;
      setTrexStatus(Status.RUNNING);
    }
  }

  function updateTrexAnimation(deltaTime) {
    if (trex.status === Status.WAITING) {
      trex.blinkTimer += deltaTime;
      if (trex.blinkTimer >= trex.blinkDelay && trex.currentFrame === 0) {
        trex.currentFrame = 1;
      }
      if (trex.blinkTimer >= trex.blinkDelay + animFrames[Status.WAITING].msPerFrame) {
        trex.currentFrame = 0;
        trex.blinkTimer = 0;
        trex.blinkDelay = randomInt(1000, 7000);
        trex.blinkCount += 1;
      }
      return;
    }

    trex.timer += deltaTime;
    if (trex.timer >= trex.msPerFrame) {
      trex.currentFrame =
        trex.currentFrame === trex.currentAnimFrames.length - 1
          ? 0
          : trex.currentFrame + 1;
      trex.timer = 0;
    }
  }

  function getTrexCollisionBoxes() {
    return trex.ducking ? trexCollisionBoxes.ducking : trexCollisionBoxes.running;
  }

  function collisionDetected(obstacle) {
    var trexBoxes = getTrexCollisionBoxes();
    for (var i = 0; i < trexBoxes.length; i += 1) {
      var trexBox = trexBoxes[i];
      var a = {
        x: trex.xPos + trexBox.x,
        y: trex.yPos + trexBox.y,
        width: trexBox.width,
        height: trexBox.height,
      };
      for (var j = 0; j < obstacle.collisionBoxes.length; j += 1) {
        var obsBox = obstacle.collisionBoxes[j];
        var b = {
          x: obstacle.xPos + obsBox.x,
          y: obstacle.yPos + obsBox.y,
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

  function updateDistanceMeter(deltaTime) {
    var nextScore = state.distanceRan
      ? Math.round(state.distanceRan * 0.025)
      : 0;
    if (nextScore !== state.score) {
      state.score = nextScore;
      publishOverlayScore(false);
    }
    if (nextScore > 0 && nextScore % 100 === 0 && nextScore !== state.lastAchievement) {
      state.lastAchievement = nextScore;
      state.achievement = true;
      state.achievementTimer = 0;
      state.achievementFlashCount = 0;
    }
    if (state.achievement) {
      state.achievementTimer += deltaTime;
      if (state.achievementTimer > 1000 / 2) {
        state.achievementTimer = 0;
        state.achievementFlashCount += 1;
      }
      if (state.achievementFlashCount > 3) {
        state.achievement = false;
      }
    }
  }

  function gameOver() {
    state.playing = false;
    state.paused = false;
    state.crashed = true;
    state.gameOverAt = getTimeStamp();
    state.highScore = Math.max(state.highScore, Math.ceil(state.distanceRan));
    trex.jumping = false;
    trex.ducking = false;
    trex.speedDrop = false;
    setTrexStatus(Status.CRASHED);
    publishOverlayScore(true);
  }

  function update(deltaTime) {
    if (!spriteReady) {
      return;
    }

    if (state.playing && !state.paused && !state.crashed) {
      if (trex.jumping) {
        updateTrexJump(deltaTime);
      }
      state.runningTime += deltaTime;
      var hasObstacles = state.runningTime > CONFIG.clearTime;
      updateClouds(deltaTime, state.currentSpeed);
      updateHorizonLine(deltaTime, state.currentSpeed);
      if (hasObstacles) {
        updateObstacles(deltaTime, state.currentSpeed);
      }

      var firstObstacle = world.obstacles[0];
      if (hasObstacles && firstObstacle && collisionDetected(firstObstacle)) {
        gameOver();
      } else {
        state.distanceRan += state.currentSpeed * deltaTime / FRAME_TIME;
        if (state.currentSpeed < CONFIG.maxSpeed) {
          state.currentSpeed += CONFIG.acceleration;
        }
        updateDistanceMeter(deltaTime);
      }
      updateTrexAnimation(deltaTime);
      return;
    }

    if (state.paused || state.crashed) {
      updateTrexAnimation(deltaTime);
      return;
    }

    if (!state.activated && trex.blinkCount < CONFIG.maxBlinkCount) {
      updateTrexAnimation(deltaTime);
    }
  }

  function drawSprite(sourceX, sourceY, sourceWidth, sourceHeight, targetX, targetY, targetWidth, targetHeight) {
    context.drawImage(
      sprite,
      sourceX,
      sourceY,
      sourceWidth,
      sourceHeight,
      targetX,
      targetY,
      targetWidth,
      targetHeight
    );
  }

  function drawBackground() {
    context.save();
    context.setTransform(layout.ratio, 0, 0, layout.ratio, 0, 0);
    context.fillStyle = BG_COLOR;
    context.fillRect(0, 0, layout.width, layout.height);
    context.restore();
  }

  function drawClouds() {
    for (var i = 0; i < world.clouds.length; i += 1) {
      var cloud = world.clouds[i];
      drawSprite(
        spriteMap.cloud.x,
        spriteMap.cloud.y,
        spriteMap.cloud.width,
        spriteMap.cloud.height,
        cloud.xPos,
        cloud.yPos,
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
        spriteMap.horizon.yPos,
        spriteMap.horizon.width,
        spriteMap.horizon.height
      );
    }
  }

  function drawObstacle(obstacle) {
    if (obstacle.type === "pterodactyl") {
      drawSprite(
        spriteMap.pterodactyl.x + spriteMap.pterodactyl.width * obstacle.frame,
        spriteMap.pterodactyl.y,
        spriteMap.pterodactyl.width,
        spriteMap.pterodactyl.height,
        obstacle.xPos,
        obstacle.yPos,
        spriteMap.pterodactyl.width,
        spriteMap.pterodactyl.height
      );
      return;
    }

    var map = spriteMap[obstacle.type];
    var sourceX =
      map.x + (map.width * obstacle.size) * (0.5 * (obstacle.size - 1));
    drawSprite(
      sourceX,
      map.y,
      map.width * obstacle.size,
      map.height,
      obstacle.xPos,
      obstacle.yPos,
      obstacle.width,
      obstacle.height
    );
  }

  function drawTrex() {
    var frameOffset = trex.currentAnimFrames[trex.currentFrame] || 0;
    var sourceWidth = TREX_CONFIG.width;
    var targetWidth = TREX_CONFIG.width;
    if (trex.status === Status.DUCKING) {
      sourceWidth = TREX_CONFIG.widthDuck;
      targetWidth = TREX_CONFIG.widthDuck;
    }
    drawSprite(
      spriteMap.trex.x + frameOffset,
      spriteMap.trex.y,
      sourceWidth,
      TREX_CONFIG.height,
      trex.xPos,
      trex.yPos,
      targetWidth,
      TREX_CONFIG.height
    );
  }

  function drawDistanceDigits(value, x, y, alpha) {
    var digits = ("00000" + Math.max(0, value)).slice(-5);
    context.save();
    context.globalAlpha = alpha;
    for (var i = 0; i < digits.length; i += 1) {
      drawSprite(
        spriteMap.text.x + spriteMap.text.width * Number(digits[i]),
        spriteMap.text.y,
        spriteMap.text.width,
        spriteMap.text.height,
        x + i * spriteMap.text.advance,
        y,
        spriteMap.text.width,
        spriteMap.text.height
      );
    }
    context.restore();
  }

  function drawHighScore() {
    var highScore = Math.round(state.highScore * 0.025);
    if (!highScore) {
      return;
    }
    var scoreX = dimensions.width - spriteMap.text.advance * 6;
    var highScoreX = scoreX - 100;
    context.save();
    context.globalAlpha = 0.8;
    drawSprite(
      spriteMap.text.x + spriteMap.text.width * 10,
      spriteMap.text.y,
      spriteMap.text.width,
      spriteMap.text.height,
      highScoreX,
      5,
      spriteMap.text.width,
      spriteMap.text.height
    );
    drawSprite(
      spriteMap.text.x + spriteMap.text.width * 11,
      spriteMap.text.y,
      spriteMap.text.width,
      spriteMap.text.height,
      highScoreX + spriteMap.text.advance,
      5,
      spriteMap.text.width,
      spriteMap.text.height
    );
    drawDistanceDigits(highScore, highScoreX + spriteMap.text.advance * 3, 5, 1);
    context.restore();
  }

  function drawDistanceMeter() {
    var scoreX = dimensions.width - spriteMap.text.advance * 6;
    var paint = !state.achievement || state.achievementFlashCount % 2 === 0;
    if (paint) {
      drawDistanceDigits(state.score, scoreX, 5, 1);
    }
    drawHighScore();
  }

  function drawGameOverPanel() {
    var centerX = dimensions.width / 2;
    var textWidth = 191;
    var textHeight = 11;
    var textTargetX = Math.round(centerX - textWidth / 2);
    var textTargetY = Math.round((dimensions.height - 25) / 3);
    drawSprite(
      spriteMap.text.x,
      spriteMap.text.y + 13,
      textWidth,
      textHeight,
      textTargetX,
      textTargetY,
      textWidth,
      textHeight
    );
    drawSprite(
      spriteMap.restart.x,
      spriteMap.restart.y,
      spriteMap.restart.width,
      spriteMap.restart.height,
      centerX - spriteMap.restart.height / 2,
      dimensions.height / 2,
      spriteMap.restart.width,
      spriteMap.restart.height
    );
  }

  function drawPausedPrompt() {
    context.save();
    context.fillStyle = "#535353";
    context.font = "12px 'Microsoft YaHei', sans-serif";
    context.textAlign = "center";
    context.fillText("PAUSED", dimensions.width / 2, 74);
    context.restore();
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
    context.imageSmoothingEnabled = false;

    drawClouds();
    drawHorizon();
    for (var i = 0; i < world.obstacles.length; i += 1) {
      drawObstacle(world.obstacles[i]);
    }
    drawTrex();
    drawDistanceMeter();
    if (state.crashed) {
      drawGameOverPanel();
    } else if (state.paused) {
      drawPausedPrompt();
    }

    context.restore();
  }

  function snapshot() {
    return {
      score: state.score,
      bestScore: Math.round(state.highScore * 0.025),
      runActive: state.playing,
      runPaused: state.paused,
      crashed: state.crashed,
      obstacleCount: world.obstacles.length,
      speed: Number(state.currentSpeed.toFixed(2)),
      hostReady: hostReady(),
    };
  }

  function frame(now) {
    if (!world.lastTime) {
      world.lastTime = now;
    }
    var deltaTime = Math.min(100, now - world.lastTime);
    world.lastTime = now;
    update(deltaTime);
    drawScene();
    world.frameHandle = window.requestAnimationFrame(frame);
  }

  function bindEvents() {
    window.addEventListener("resize", function () {
      adjustDimensions();
      drawScene();
    });
    window.addEventListener("keydown", function (event) {
      if (event.code === "Space" || event.code === "ArrowUp") {
        event.preventDefault();
        handleJumpInput();
        return;
      }
      if (event.code === "ArrowDown") {
        event.preventDefault();
        if (trex.jumping) {
          setSpeedDrop();
          return;
        }
        setDuck(true);
        return;
      }
      if (event.code === "Enter" && state.crashed) {
        event.preventDefault();
        handleJumpInput();
        return;
      }
      if (event.code === "KeyP") {
        event.preventDefault();
        togglePause();
      }
    });
    window.addEventListener("keyup", function (event) {
      if (event.code === "Space" || event.code === "ArrowUp") {
        endJump();
        return;
      }
      if (event.code === "ArrowDown") {
        trex.speedDrop = false;
        setDuck(false);
      }
    });
    canvas.addEventListener("pointerdown", function (event) {
      event.preventDefault();
      world.pointerActive = true;
      handleJumpInput();
    });
    canvas.addEventListener("pointerup", function (event) {
      event.preventDefault();
      if (world.pointerActive) {
        endJump();
      }
      world.pointerActive = false;
    });
    canvas.addEventListener("pointercancel", function () {
      world.pointerActive = false;
      endJump();
    });
    document.addEventListener("visibilitychange", function () {
      if (document.hidden) {
        setPaused(true);
      }
    });
    window.addEventListener("blur", function () {
      setPaused(true);
    });
  }

  adjustDimensions();
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
      return snapshot();
    },
    startRun: function () {
      startRun(false);
      return snapshot();
    },
  };

  world.frameHandle = window.requestAnimationFrame(frame);
})();

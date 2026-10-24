/// Compiles JavaScript UI regions into shared host-executed layout programs.
var OperitComposeCompiler = function () {
  /// Lowers source inputs and validates already lowered artifacts through the same ABI.
  function prepare(source, identity) {
    var compiled = OperitComposeArtifact.read(source);
    return compiled === null ? OperitComposeCompiler.compile(source, identity) : compiled;
  }
  /// Emits ordinary portable JS; no runtime objects, bytecode or user state are captured.
  function artifact(source, identity) {
    var compiled = OperitComposeArtifact.read(source);
    return compiled === null ? OperitComposeArtifact.prefix + OperitComposeArtifact.version + '\n' + compile(source, identity) : compiled;
  }
  var childNodes = new WeakMap();
  /// Visits direct ESTree child nodes without traversing locations or primitive properties.
  function children(node) {
    if (childNodes.has(node)) return childNodes.get(node);
    var result = [];
    Object.keys(node).forEach(function (key) {
      var value = node[key];
      if (Array.isArray(value)) value.forEach(function (item) {
        if (item && typeof item.type === 'string') result.push(item);
      });else if (value && typeof value.type === 'string') result.push(value);
    });
    result.sort(function (a, b) {
      return a.start - b.start;
    });
    childNodes.set(node, result);
    return result;
  }

  /// Recognizes the existing context.UI.Type call using its parsed member structure.
  function uiCall(node) {
    if (node.type !== 'CallExpression' || node.optional) return null;
    var member = node.callee;
    if (member.type !== 'MemberExpression' || member.computed || member.optional) return null;
    var registry = member.object;
    if (registry.type !== 'MemberExpression' || registry.computed || registry.optional || registry.property.name !== 'UI') return null;
    return registry.object.type === 'Identifier' ? registry.object : null;
  }

  /// Transforms only parsed UI construction calls and leaves user control flow and side effects intact.
  function compile(source, identity) {
    var compiled = OperitComposeArtifact.read(source);
    if (compiled !== null) return compiled;
    var ast = __operitAcorn.parse(source, {
      ecmaVersion: 2022,
      sourceType: 'script',
      allowReturnOutsideFunction: true
    });
    var programs = [];
    var programPrefix = '__operitUiPrograms_';
    while (source.indexOf(programPrefix) !== -1) programPrefix += '_';
    var hash = 2166136261;
    for (var offset = 0; offset < source.length; offset += 1) hash = Math.imul(hash ^ source.charCodeAt(offset), 16777619);
    var moduleId = identity + ':' + (hash >>> 0).toString(36);

    /// Retains observable property enumeration and accessor evaluation in JavaScript.
    function fixedObject(node) {
      var keys = new Set();
      return node.properties.every(
      /// Accepts only ordinary fields with an unambiguous enumeration order.
      function (property) {
        if (property.type !== 'Property' || property.computed || property.kind !== 'init' || property.method) return false;
        var key = property.key.type === 'Identifier' ? property.key.name : String(property.key.value);
        if (key === '__proto__' || keys.has(key) || String(Number(key) >>> 0) === key && Number(key) < 4294967295) return false;
        keys.add(key);
        return true;
      });
    }

    /// Recognizes SDK modifier calls without changing foreign receiver semantics.
    function modifierOperations(node, context) {
      var operations = [];
      var current = node;
      while (current.type === 'CallExpression' && !current.optional && current.callee.type === 'MemberExpression' && !current.callee.computed && !current.callee.optional) {
        if (current.callee.property.name === 'toJSON' || current.callee.property.name === '__modifierOps' || current.arguments.some(
        /// Preserves spread iteration in the original JavaScript expression.
        function (argument) { return argument.type === 'SpreadElement'; })) return null;
        operations.unshift({ name: current.callee.property.name, args: current.arguments });
        current = current.callee.object;
      }
      return operations.length && current.type === 'MemberExpression' && !current.computed && !current.optional && current.object.type === 'Identifier' && current.object.name === context && current.property.name === 'Modifier' ? operations : null;
    }

    /// Extracts a whole nested layout into immutable data and evaluates only its dynamic leaves.
    function lowerRegion(root, plan) {
      var values = [];
      var instructions = [];
      var context = uiCall(root).name;
      /// Records an ordinary JavaScript expression at its exact source evaluation position.
      function parameter(node) {
        var index = values.length;
        values.push(emit(node, plan));
        return [1, index];
      }
      /// Encodes literals, fixed containers, modifiers and internal nodes without runtime node objects.
      function operand(node) {
        if (node.type === 'Literal' && !node.regex && typeof node.value !== 'bigint') return [0, node.value];
        var receiver = uiCall(node);
        if (receiver && receiver.name === context && node.arguments.length <= 2 && !node.arguments.some(
        /// Keeps observable argument spreading in JavaScript.
        function (argument) { return argument.type === 'SpreadElement'; })) return [4, instruction(node)];
        if (node.type === 'ObjectExpression' && fixedObject(node)) {
          var keys = [], members = [];
          node.properties.forEach(
          /// Extracts fields in their original source order.
          function (property) {
            keys.push(property.key.type === 'Identifier' ? property.key.name : String(property.key.value));
            members.push(operand(property.value));
          });
          return [2, keys, members];
        }
        if (node.type === 'ArrayExpression' && node.elements.every(
        /// Retains spread evaluation as an ordinary dynamic value.
        function (element) { return !element || element.type !== 'SpreadElement'; })) return [3, node.elements.map(
        /// Represents holes as null because they cannot contribute UI children.
        function (element) { return element ? operand(element) : [0, null]; })];
        var operations = modifierOperations(node, context);
        if (operations) {
          var expression = 'OperitComposeNodes.modifierBase(' + context + '.Modifier)';
          operations.forEach(
          /// Freezes each fluent receiver at its original call boundary before later arguments run.
          function (operation) {
            expression += '.concat([{name:' + JSON.stringify(operation.name) + ',args:[' + operation.args.map(
            /// Preserves argument side effects without allocating modifier proxy chains.
            function (argument) { return emit(argument, plan); }).join(',') + ']}])';
          });
          var modifierIndex = values.length;
          values.push('({__modifierOps:' + expression + '})');
          return [1, modifierIndex];
        }
        return parameter(node);
      }
      /// Allocates a local instruction index, never a runtime source or mounted identity.
      function instruction(node) {
        var index = instructions.length;
        instructions.push(null);
        var props = node.arguments[0] ? operand(node.arguments[0]) : [0, {}];
        var children = node.arguments[1] ? operand(node.arguments[1]) : [0, null];
        instructions[index] = { type: node.callee.property.name, props: props, children: children, explicitChildren: node.arguments.length === 2 };
        return index;
      }
      var rootIndex = instruction(root);
      var index = programs.length;
      programs.push({ id: moduleId + ':' + root.start, root: rootIndex, nodes: instructions });
      return { program: programPrefix + '[' + index + ']', values: '[' + values.join(',') + ']' };
    }

    /// Recognizes a statically keyed state binding whose lexical value can be refreshed independently.
    function stateBinding(declaration, context) {
      var call = declaration.init;
      return declaration.id.type === 'ArrayPattern' && declaration.id.elements.length <= 2 && declaration.id.elements.every(function (item) {
        return item === null || item.type === 'Identifier';
      }) && call && call.type === 'CallExpression' && call.callee.type === 'MemberExpression' && !call.callee.computed && call.callee.object.type === 'Identifier' && call.callee.object.name === context && call.callee.property.name === 'useState' && call.arguments[0] && call.arguments[0].type === 'Literal' && typeof call.arguments[0].value === 'string';
    }

    /// Keeps observable calls, mutations and control-flow effects inside their original execution unit.
    function separable(node) {
      if (!node) return true;
      if (node.type === 'FunctionExpression' || node.type === 'ArrowFunctionExpression') return true;
      if (node.type === 'AssignmentExpression' || node.type === 'UpdateExpression' || node.type === 'AwaitExpression' || node.type === 'YieldExpression' || node.type === 'NewExpression' || node.type === 'TaggedTemplateExpression') return false;
      if (node.type === 'UnaryExpression' && node.operator === 'delete') return false;
      if (node.type === 'CallExpression') {
        if (!uiCall(node) && !(node.callee.type === 'MemberExpression' && !node.callee.computed && node.callee.property.name === 'map')) return false;
      }
      return children(node).every(separable);
    }

    /// Rejects captured variable assignments that lack an observable state-write boundary.
    function capturedWrites(node, names) {
      if (!node) return false;
      if (node.type === 'AssignmentExpression' || node.type === 'UpdateExpression') {
        var target = node.type === 'AssignmentExpression' ? node.left : node.argument;
        if (target.type === 'Identifier' && names.has(target.name)) return true;
      }
      return children(node).some(function (child) {
        return capturedWrites(child, names);
      });
    }

    /// Partitions straight-line synchronous render functions into lexical binding and expression units.
    function renderFrame(fn) {
      if (fn.async || fn.generator || fn.params.length !== 1 || fn.params[0].type !== 'Identifier' || fn.body.type !== 'BlockStatement') return null;
      var context = fn.params[0].name;
      var statements = fn.body.body;
      if (!statements.length || statements[statements.length - 1].type !== 'ReturnStatement') return null;
      var hasState = false;
      var variables = new Set();
      for (var index = 0; index < statements.length - 1; index += 1) {
        var statement = statements[index];
        if (statement.type === 'FunctionDeclaration') continue;
        if (statement.type !== 'VariableDeclaration') return null;
        for (var declaration of statement.declarations) {
          if (stateBinding(declaration, context)) {
            hasState = true;
            if (declaration.id.elements[0]) variables.add(declaration.id.elements[0].name);
          } else if (declaration.id.type === 'Identifier' && declaration.init && separable(declaration.init)) {
            if (declaration.init.type !== 'ArrowFunctionExpression' && declaration.init.type !== 'FunctionExpression') variables.add(declaration.id.name);
          } else return null;
        }
      }
      if (!hasState || !uiCall(statements[statements.length - 1].argument) || !separable(statements[statements.length - 1].argument)) return null;
      if (capturedWrites(fn.body, variables)) return null;
      return {
        context: context,
        variables: variables,
        statements: statements,
        frame: '__operitFrame_' + fn.start
      };
    }

    /// Finds lexical reads used by this execution unit, excluding independently registered UI expressions and callbacks.
    function reads(node, plan, output) {
      if (!node || node.type === 'FunctionExpression' || node.type === 'ArrowFunctionExpression') return;
      if (node.type === 'Identifier') {
        if (plan.variables.has(node.name)) output.add(node.name);
        return;
      }
      if (node.type === 'MemberExpression') {
        reads(node.object, plan, output);
        if (node.computed) reads(node.property, plan, output);
        return;
      }
      if (node.type === 'Property') {
        if (node.computed) reads(node.key, plan, output);
        reads(node.value, plan, output);
        return;
      }
      children(node).forEach(function (child) {
        reads(child, plan, output);
      });
    }

    /// Emits dependency keys bound to the exact captured lexical frame.
    function readKeys(node, plan) {
      var names = new Set();
      reads(node, plan, names);
      return '[' + Array.from(names).map(function (name) {
        return plan.frame + '.key+' + JSON.stringify(':value:' + name);
      }).join(',') + ']';
    }

    /// Preserves captured lexical variables while registering their independent refresh functions.
    function emitFrame(fn, plan) {
      var output = source.slice(fn.start, fn.body.start) + '{\nconst ' + plan.frame + '=OperitComposeReactive.frame(' + plan.context + ',' + JSON.stringify(identity + ':' + fn.start) + ');\n';
      for (var index = 0; index < plan.statements.length - 1; index += 1) {
        var statement = plan.statements[index];
        if (statement.type === 'FunctionDeclaration') {
          output += emit(statement) + '\n';
          continue;
        }
        for (var declaration of statement.declarations) {
          if (stateBinding(declaration, plan.context)) {
            var name = declaration.id.elements[0];
            output += 'let ' + source.slice(declaration.id.start, declaration.id.end) + '=' + source.slice(declaration.init.start, declaration.init.end) + ';\n';
            if (name) output += 'OperitComposeReactive.state(' + plan.frame + ',' + JSON.stringify(name.name) + ',' + source.slice(declaration.init.arguments[0].start, declaration.init.arguments[0].end) + ',(__value)=>{' + name.name + '=__value;});\n';
          } else if (declaration.init.type === 'ArrowFunctionExpression' || declaration.init.type === 'FunctionExpression') {
            output += 'const ' + declaration.id.name + '=' + emit(declaration.init) + ';\n';
          } else {
            output += 'let ' + declaration.id.name + ';OperitComposeReactive.derived(' + plan.frame + ',' + JSON.stringify(declaration.id.name) + ',' + readKeys(declaration.init, plan) + ',()=>(' + emit(declaration.init, plan) + '),(__value)=>{' + declaration.id.name + '=__value;});\n';
          }
        }
      }
      return output + 'return ' + emit(plan.statements[plan.statements.length - 1].argument, plan) + ';\n}';
    }

    /// Materializes a source range only when an enclosing rewrite needs its complete text.
    function emit(node, plan) {
      var rewritten = rewrite(node, plan);
      return rewritten === null ? source.slice(node.start, node.end) : rewritten;
    }

    /// Produces replacements only for AST ranges that contain an actual DSL transformation.
    function rewrite(node, plan) {
      if (node.type === 'FunctionDeclaration' || node.type === 'FunctionExpression' || node.type === 'ArrowFunctionExpression') {
        var partition = renderFrame(node);
        if (partition) return emitFrame(node, partition);
      }
      var receiver = uiCall(node);
      if (receiver) {
        var context = source.slice(receiver.start, receiver.end);
        var site = JSON.stringify(identity + ':' + node.start);
        if (node.arguments.length > 2 || node.arguments.some(
        /// Leaves dynamic argument iteration in the public construction API.
        function (argument) { return argument.type === 'SpreadElement'; })) {
          var args = node.arguments.map(
          /// Lowers nested UI without changing the surrounding call contract.
          function (argument) { return emit(argument, plan); }).join(',');
          var dynamic = source.slice(node.callee.start, node.callee.end) + '(' + args + ')';
          return plan ? 'OperitComposeReactive.expressionDynamic(' + plan.frame + ',' + site + ',' + readKeys(node, plan) + ',()=>(' + dynamic + '))' : dynamic;
        }
        var region = lowerRegion(node, plan);
        if (plan) return 'OperitComposeReactive.expressionProgram(' + plan.frame + ',' + site + ',' + readKeys(node, plan) + ',()=>'+ region.values + ',' + region.program + ')';
        return 'OperitComposeComposition.evaluate(' + context + ',' + site + ',' + region.program + ',' + region.values + ')';
      }

      var cursor = node.start;
      var visitedEnd = node.start;
      var output = null;
      children(node).forEach(function (child) {
        if (child.start < visitedEnd) return;
        visitedEnd = child.end;
        var replacement = rewrite(child, plan);
        if (replacement === null) return;
        if (output === null) output = '';
        output += source.slice(cursor, child.start) + replacement;
        cursor = child.end;
      });
      return output === null ? null : output + source.slice(cursor, node.end);
    }
    var lowered = emit(ast);
    var prologueEnd = 0;
    for (var statementIndex = 0; statementIndex < ast.body.length && ast.body[statementIndex].directive; statementIndex += 1) prologueEnd = ast.body[statementIndex].end;
    var prefix = 'var ' + programPrefix + '=' + JSON.stringify(programs) + ';\n';
    return lowered.slice(0, prologueEnd) + '\n' + prefix + lowered.slice(prologueEnd);
  }
  return {
    compile: compile,
    prepare: prepare,
    artifact: artifact,
    artifactVersion: OperitComposeArtifact.version
  };
}();

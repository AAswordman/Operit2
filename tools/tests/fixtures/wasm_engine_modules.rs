use std::borrow::Cow;
use wasm_encoder::*;

fn body(instructions: &[Instruction<'_>]) -> Function {
    let mut function = Function::new([]);
    for instruction in instructions {
        function.instruction(instruction);
    }
    function.instruction(&Instruction::End);
    function
}

pub fn scalar_module() -> Vec<u8> {
    use Instruction::*;
    let mut module = Module::new();
    let mut types = TypeSection::new();
    types.ty().function([ValType::I32], [ValType::I32]);
    types.ty().function([ValType::I64], [ValType::I64]);
    types.ty().function([ValType::F32], [ValType::F32]);
    types.ty().function([ValType::F64], [ValType::F64]);
    types
        .ty()
        .function([ValType::EXTERNREF], [ValType::EXTERNREF]);
    types
        .ty()
        .function([ValType::EXTERNREF], [ValType::EXTERNREF, ValType::I32]);
    let mut imports = ImportSection::new();
    imports.import("env", "identity", EntityType::Function(4));
    imports.import("env", "multi", EntityType::Function(5));
    let mut functions = FunctionSection::new();
    for ty in [0, 1, 2, 3, 4, 5, 0, 0, 0, 0, 0, 0, 0, 0] {
        functions.function(ty);
    }
    let mut tables = TableSection::new();
    tables.table(TableType {
        element_type: RefType::FUNCREF,
        table64: false,
        minimum: 1,
        maximum: Some(1),
        shared: false,
    });
    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: 1,
        maximum: Some(3),
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    let mut exports = ExportSection::new();
    for (index, name) in [
        "store", "i64", "f32", "f64", "identity", "multi", "indirect", "grow", "trap", "double",
        "simd", "load", "storeAt", "loadSimd",
    ]
    .iter()
    .enumerate()
    {
        exports.export(name, ExportKind::Func, index as u32 + 2);
    }
    exports.export("memory", ExportKind::Memory, 0);
    let mut elements = ElementSection::new();
    elements.active(
        None,
        &ConstExpr::i32_const(0),
        Elements::Functions(Cow::Borrowed(&[11])),
    );
    let memory = MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    };
    let mut code = CodeSection::new();
    for instructions in [
        vec![
            I32Const(0),
            LocalGet(0),
            I32Store(memory),
            I32Const(0),
            I32Load(memory),
        ],
        vec![LocalGet(0)],
        vec![LocalGet(0), F32Const(0.5f32.into()), F32Mul],
        vec![LocalGet(0), F64Const(0.25f64.into()), F64Add],
        vec![LocalGet(0), Call(0)],
        vec![LocalGet(0), Call(1)],
        vec![
            LocalGet(0),
            I32Const(0),
            CallIndirect {
                type_index: 0,
                table_index: 0,
            },
        ],
        vec![LocalGet(0), MemoryGrow(0)],
        vec![Unreachable],
        vec![LocalGet(0), I32Const(2), I32Mul],
        vec![LocalGet(0), I32x4Splat, I32x4ExtractLane(2)],
        vec![LocalGet(0), I32Load(memory)],
        vec![LocalGet(0), I32Const(0x12345678), I32Store(memory), I32Const(1)],
        vec![LocalGet(0), V128Load(MemArg { align: 0, ..memory }), I32x4ExtractLane(0)],
    ] {
        code.function(&body(&instructions));
    }
    module
        .section(&types)
        .section(&imports)
        .section(&functions)
        .section(&tables)
        .section(&memories)
        .section(&exports)
        .section(&elements)
        .section(&code);
    module.finish()
}

pub fn reference_module() -> Vec<u8> {
    use Instruction::*;
    let mut module = Module::new();
    let mut types = TypeSection::new();
    types.ty().struct_([FieldType {
        element_type: StorageType::Val(ValType::EXTERNREF),
        mutable: true,
    }]);
    types
        .ty()
        .array(&StorageType::Val(ValType::EXTERNREF), true);
    let structure = ValType::Ref(RefType {
        nullable: false,
        heap_type: HeapType::Concrete(0),
    });
    let array = ValType::Ref(RefType {
        nullable: false,
        heap_type: HeapType::Concrete(1),
    });
    types.ty().function([ValType::EXTERNREF], [structure]);
    types.ty().function([structure, ValType::EXTERNREF], []);
    types.ty().function([structure], [ValType::EXTERNREF]);
    types.ty().function([], [array]);
    types.ty().function([array, ValType::EXTERNREF], []);
    types
        .ty()
        .function([array, ValType::I32], [ValType::EXTERNREF]);
    let mut functions = FunctionSection::new();
    for ty in 2..8 {
        functions.function(ty);
    }
    let mut exports = ExportSection::new();
    for (index, name) in [
        "newStruct",
        "setStruct",
        "getStruct",
        "newArray",
        "fillArray",
        "getArray",
    ]
    .iter()
    .enumerate()
    {
        exports.export(name, ExportKind::Func, index as u32);
    }
    let mut code = CodeSection::new();
    for instructions in [
        vec![LocalGet(0), StructNew(0)],
        vec![
            LocalGet(0),
            LocalGet(1),
            StructSet {
                struct_type_index: 0,
                field_index: 0,
            },
        ],
        vec![
            LocalGet(0),
            StructGet {
                struct_type_index: 0,
                field_index: 0,
            },
        ],
        vec![I32Const(4), ArrayNewDefault(1)],
        vec![
            LocalGet(0),
            I32Const(0),
            LocalGet(1),
            I32Const(4),
            ArrayFill(1),
        ],
        vec![LocalGet(0), LocalGet(1), ArrayGet(1)],
    ] {
        code.function(&body(&instructions));
    }
    module
        .section(&types)
        .section(&functions)
        .section(&exports)
        .section(&code);
    module.finish()
}

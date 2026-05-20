use boa_engine::{
    js_string, Context, JsValue, Source,
    object::ObjectInitializer,
    property::Attribute,
};

#[derive(Debug)]
pub struct LayoutNode {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl LayoutNode {
    pub fn to_js_object(&self, ctx: &mut Context) -> JsValue {
        // FIX: js_string!(...) für alle Property-Keys und String-Werte
        let obj = ObjectInitializer::new(ctx)
            .property(js_string!("id"),     js_string!(self.id.as_str()), Attribute::all())
            .property(js_string!("x"),      self.x as f64,                Attribute::all())
            .property(js_string!("y"),      self.y as f64,                Attribute::all())
            .property(js_string!("width"),  self.width as f64,            Attribute::all())
            .property(js_string!("height"), self.height as f64,           Attribute::all())
            .build();
        JsValue::from(obj)
    }
}

pub fn run() {
    let mut ctx = Context::default();

    let nodes = vec![
        LayoutNode { id: "header".into(), x: 0.0,  y: 0.0,   width: 800.0, height: 80.0 },
        LayoutNode { id: "main".into(),   x: 16.0, y: 96.0,  width: 768.0, height: 40.0 },
        LayoutNode { id: "footer".into(), x: 0.0,  y: 152.0, width: 800.0, height: 60.0 },
    ];

    let js_array = boa_engine::object::builtins::JsArray::new(&mut ctx);
    for node in &nodes {
        let js_obj = node.to_js_object(&mut ctx);
        js_array.push(js_obj, &mut ctx).unwrap();
    }

    ctx.global_object().set(
        js_string!("layoutTree"),
        js_array,
        false, &mut ctx,
    ).unwrap();

    let result = ctx.eval(Source::from_bytes(r#"
        const totalHeight = layoutTree.reduce((sum, node) => sum + node.height, 0);
        const ids = layoutTree.map(n => n.id).join(", ");
        ({
            nodeCount:   layoutTree.length,
            totalHeight: totalHeight,
            ids:         ids,
            shifted:     layoutTree.map(n => ({ id: n.id, x: n.x + 10 }))
        })
    "#)).expect("JS-Fehler");

    let obj = result.as_object().unwrap();

    // FIX: js_string!(...) für alle .get()-Aufrufe
    let count = obj.get(js_string!("nodeCount"),   &mut ctx).unwrap();
    let total = obj.get(js_string!("totalHeight"), &mut ctx).unwrap();
    let ids   = obj.get(js_string!("ids"),         &mut ctx).unwrap();

    println!("  Knoten:     {}", count.display());
    println!("  Gesamthöhe: {}px", total.display());
    println!("  IDs:        {}", ids.as_string().unwrap().to_std_string_escaped());

    let shifted_arr = obj.get(js_string!("shifted"), &mut ctx).unwrap();
    let shifted_arr = shifted_arr.as_object().unwrap();
    let len = shifted_arr.get(js_string!("length"), &mut ctx).unwrap().as_number().unwrap() as usize;

    println!("  Verschobene x-Werte:");
    for i in 0..len {
        let item = shifted_arr.get(i, &mut ctx).unwrap();
        let item_obj = item.as_object().unwrap();
        let id = item_obj.get(js_string!("id"), &mut ctx).unwrap()
            .as_string().unwrap().to_std_string_escaped();
        let x = item_obj.get(js_string!("x"), &mut ctx).unwrap().as_number().unwrap();
        println!("    #{}: x = {}px", id, x);
    }
}
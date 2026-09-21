use tree_sitter::{Parser, TreeCursor};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::fs::File;
use std::path::Path;
use rand::seq::SliceRandom;
use rand::{thread_rng};
use std::time::Instant;
use std::time::Duration;


//node names in the parsing tree
const CONSTANT: &str = "constant";
const IDENTIFIER: &str = "identifier";
const OP : &str = "op";
const AND : &str = "and";

#[derive(Clone)]
#[derive(Debug)]
enum VariableState {
    AtLowerBound,
    AtUpperBound,
    EqualBound,
    Free,
}

#[derive(Clone)]
#[derive(Debug)]
enum Sorting {
    Normal,
    Reversed,
    Random,
}


fn main() -> Result<(), String> {

    // init parser and language
    let mut parser = Parser::new();
    let language = tree_sitter_input_simplex::LANGUAGE;
    parser.set_language(&language.into()).expect("Error loading InputSimplex parser");
    
    let (input, sorting) = menu();

    
    println!("\n[@] Processing file...");
    match process_file(&input, parser, sorting) {
        Ok(duration) => {
            println!("\n----------------------------------------");
            println!("[✓] File processed successfully in {} ms", duration.as_millis());
        }
        Err(e) => {
            eprintln!("\n[X] Error reading file: {}", e);
        }
    }
    

    Ok(())
}

fn menu() -> (String, Sorting) {

    println!("\n\n\n");
    println!("========================================");
    println!("      General Simplex Solver CLI        ");
    println!("========================================");
    println!("\n\n");

    let filename = get_input("Enter the path to your input file: ");
    let sorting = select_sorting();
    
    println!("\n[!] Configuration loaded.");
    println!(" - File: {}", filename);
    println!(" - Sorting: {:?}", sorting);
    println!("----------------------------------------");
    
    (filename, sorting)
}

fn get_input(
prompt: &str
) -> String {

    let mut input = String::new();
    print!("{}", prompt);
    
    io::stdout().flush().unwrap(); 
    io::stdin().read_line(&mut input).expect("Failed to read line");
    
    input.trim().to_string()
}

fn select_sorting(
) -> Sorting {
    loop {
        println!("\nSelect sorting for variables:");
        println!("  1. Normal (s1 < s2 < s3 < x1 < x2 ...)");
        println!("  2. Inverted (x2 < x1 < s3 < s2 < s1 ...)");
        println!("  3. Random");
        
        let choice = get_input("Enter choice (1-3): ");
        
        match choice.as_str() {
            "1" => return Sorting::Normal,
            "2" => return Sorting::Reversed,
            "3" => return Sorting::Random,
            _ => println!("X Invalid choice. Please enter 1, 2, or 3."),
        }
    }
}

fn process_file(
filename: &str,
mut parser: Parser,
sorting: Sorting
) -> Result<Duration, Box<dyn std::error::Error>> {
    let path = Path::new(filename);
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    
    let mut output_file = File::create("outputs/output.txt")?;
    let mut eq_num: i32 = 0;
    
    //timer for measuring performances
    let mut total_duration = Duration::new(0, 0);

    for (_index, line) in reader.lines().enumerate() {
        let mut start = Instant::now();
    
        let line = line?;
        let source = line.trim();

        if source.is_empty() || source.starts_with('#') {
            continue;
        }
    
        let tree = parser.parse(&source, None).unwrap(); 
        assert!(!tree.root_node().has_error());
        let root = tree.root_node();
        
        let mut vars: HashMap<String, usize> = HashMap::new();
        //the param is the key, the top value is the value
        let mut params: HashMap<String, i32> = HashMap::new();
        let mut cursor = root.walk(); 
        

        params_discover(&mut cursor, &source, &mut params);
        cursor = root.walk(); // resets cursor
        
        //extract number of ands, populate vars hashmap
        let mut rows = vars_discover(&mut cursor, &source, &mut vars, &mut params) + 1;
        let cols: i32 = vars.len().try_into().unwrap();
        cursor = root.walk(); // resets cursor
        
        //stops timer
        total_duration += start.elapsed();
        
        write!(output_file, "##################################### {} #####################################\n\n", eq_num)?;
        
        start = Instant::now();
        
        let mut params_combinations: Vec<HashMap<String, f64>> = vec![];
        let mut params_current: HashMap<String, f64> = HashMap::new();
        let params_max: Vec<(String, i32)> = params.clone().into_iter().collect();
        let mut num_combinations = 1;
        
        //calculate all possible combinations 
        for i in params.keys() {
            let Some(value) = params.get(i) else { todo!()};
            params_current.insert(i.to_string(), 0.0);
            num_combinations *= value + 1;
        }
        
        
        if num_combinations != 1 {
            params_current = HashMap::new();
            generate_combinations(0, &mut params_current, &params_max, &mut params_combinations);
            rows -= 1;
        }
        
        
        //at this level you iterate on all possible combinations of parameters
        for i in 0..num_combinations as usize{ //PxPxPxP...
            let mut matrix: Vec<f64> = vec![0.0;(rows*cols).try_into().unwrap()];
            let state: VariableState = VariableState::Free;
            let mut left: f64 = 1.0;
            let mut num_eq : i32 = 0;
            
            //the first "cols" cells are dedicated to basic variables. the others "rows" to non basic
            let mut states: Vec<(VariableState, f64)> = vec![(VariableState::Free,0.0);(rows+cols).try_into().unwrap()];
            cursor = root.walk(); // resets cursor
            
            if num_combinations != 1 {
                dfs_mod(&mut cursor, &source, &mut left, &mut states, &mut vars, &mut matrix, &mut num_eq, state.clone(), false, params_combinations[i].clone());
            } else {
                //params servses as a placeholder it will not be used
                dfs_mod(&mut cursor, &source, &mut left, &mut states, &mut vars, &mut matrix, &mut num_eq, state.clone(), false, params_current.clone());
            }
            
            
            total_duration += start.elapsed();
            
            
            if params_combinations.len() > 0 {
                write!(output_file, "==================== {} ======================\n\n", i)?;
                write!(output_file, "Parameters: {:?}\n", params_combinations[i])?;
            } else {
                write!(output_file, "=============================================\n\n")?;
            }
            total_duration += simplex(&mut states, vars.clone(), &mut matrix, &mut output_file, sorting.clone())?;
            start = Instant::now();
        }
        
        total_duration += start.elapsed();
        write!(output_file, "#############################################################################\n\n")?;
        eq_num += 1;
        start = Instant::now();
    }
    
    
    Ok(total_duration)
}


fn generate_combinations(
    index: usize,
    current: &mut HashMap<String, f64>,
    max_values: &Vec<(String, i32)>,
    result: &mut Vec<HashMap<String, f64>>,
) {
    if index == max_values.len() {
        result.push(current.clone());
        return;
    }

    let (key, max) = &max_values[index];

    for i in 0..=*max {
        current.insert(key.clone(), i as f64);
        generate_combinations(index + 1, current, max_values, result);
    }
}

fn simplex(
mut ranges: &mut Vec<(VariableState,f64)>, 
mut vars: HashMap<String, usize>, 
matrix: &mut Vec<f64>,
output_file: &mut File, 
sorting: Sorting
) -> Result<Duration, Box<dyn std::error::Error>> {

    let start = Instant::now();
    let mut total_dur = Duration::new(0,0);
    let non_basics_len = vars.len();
    let basics_len = ranges.len()-non_basics_len;
    
    // first basic vars and then non basic
    let mut variables: Vec<String> = vec![String::new(); basics_len + non_basics_len];
    
    //initial assignments to all variables
    let mut stock: Vec<f64> = vec![0.0;basics_len + non_basics_len];

    
    //index memory of positions
    let mut memory: Vec<(String, usize)> = vec![(String::new(),0);non_basics_len];
    
    //transfer variables from hashmap to vector
    for i in vars.keys() {
        let Some(index) = vars.get(i) else { todo!()};
        variables[index + basics_len] = i.to_string();
        memory[*index] = (i.to_string(),index + basics_len);
    }
    
    let mut basic: HashMap<String, usize> = HashMap::new();
    
    //give names to basic variables
    for i in 0..basics_len {
        variables[i] = i.to_string();
        basic.entry(i.to_string()).or_insert(i);
    }
    
    let mut order: Vec<String> = variables.clone();
    let mut rng = thread_rng();

    
    let _sort = match sorting {
        Sorting::Normal => {},
        Sorting::Reversed => order.reverse(),
        Sorting::Random => order.shuffle(&mut rng),
    };
        
    
    //recover the violation of bounds and other infor (row violation/ basic variables)
    let (mut pos, mut flag, mut bound, mut bound_type) = check_range(&mut ranges, &mut stock, order.clone(), basic.clone());
    
    let mut is_satisfiable = false;
    
    if flag {
        is_satisfiable = true;
    }
    
    while !flag && pos < (basics_len) {
        
        //research suitable var and terminate if no found
        let idx = suitable_var(pos, matrix, non_basics_len, bound_type, ranges, &stock, order.clone(), &mut vars);
        
        if idx == usize::MAX {
            is_satisfiable = false;
            break;
        }
        
        //new index needed for variables array
        let index_var = basics_len + idx;
        
        let pivot_idx = pos * non_basics_len + idx;
        let coeff_pivot = matrix[pivot_idx];
        
        
        //increasing/decreasing values of stock of delta
        let delta = (bound - stock[pos])/coeff_pivot;

        stock[index_var] += delta;
        
        for i in 0..basics_len {  
            let coeff = matrix[i * non_basics_len + idx];
            stock[i] += coeff * delta;
        }
        
        // ========= PIVOT =========
        
        let mut orig_pivot_row = vec![0.0; non_basics_len];
        for j in 0..non_basics_len {
            orig_pivot_row[j] = matrix[pos * non_basics_len + j];
        }

        //update all OTHER rows first
        for i in 0..basics_len {
            if i == pos { continue; }
            
            let old_a_i_pivot = matrix[i * non_basics_len + idx];
            matrix[i * non_basics_len + idx] = old_a_i_pivot / coeff_pivot;
            
            //update all other cells in this row
            for j in 0..non_basics_len {
                if j == idx { continue; }
                matrix[i * non_basics_len + j] -= (old_a_i_pivot * orig_pivot_row[j]) / coeff_pivot;
            }
        }

        //update the pivot row
        for j in 0..non_basics_len {
            if j == idx { 
                matrix[pivot_idx] = 1.0 / coeff_pivot; 
            } else {
                matrix[pos * non_basics_len + j] = -orig_pivot_row[j] / coeff_pivot;
            }
        }

        //swaps the variables in stock basic and non basic
        let temp = stock[index_var];
        stock[index_var] = stock[pos];
        stock[pos] = temp;
        
        //swaps the ranges basic and non basic
        let (state, value) = ranges[index_var].clone();
        ranges[index_var] = ranges[pos].clone();
        ranges[pos] = (state, value);
        
        //swaps basic and non basic variables
        let var_at_pos = variables[pos].clone();
        let var_at_index = variables[index_var].clone();
        
        variables.swap(pos, index_var);

        if let Some(&index) = vars.get(&var_at_index) {
            if index < memory.len() {
                let old_string = memory[index].0.clone();
                memory[index] = (old_string, pos);
            } else {
                eprintln!("Logic Error: Variable {} mapped to index {}, but memory len is {}", var_at_index, index, memory.len());
            }
        }


        if let Some(&index) = vars.get(&var_at_pos) {
            if index < memory.len() {
                let old_string = memory[index].0.clone();
                memory[index] = (old_string, index_var);
            } else {
                eprintln!("Logic Error: Variable {} mapped to index {}, but memory len is {}", var_at_pos, index, memory.len());
            }
        }   
        
       //if let Some(&index) = vars.get(&var_at_pos) {
         //   let old_string = memory[index].0.clone();
           // memory[index] = (old_string, index_var); 
        //}
        
        vars.insert(var_at_pos.clone(), index_var - basics_len);
        vars.remove(&var_at_index);
        basic.insert(var_at_index.clone(), pos);
        basic.remove(&var_at_pos);
    
        //check again if bounds are corrects
        (pos, flag, bound, bound_type) = check_range(ranges, &mut stock, order.clone(), basic.clone());  
        
        if flag {
            is_satisfiable = true;
        }
        
        //rounding
        let threshold = 1e-9;
        for i in 0..(basics_len + non_basics_len) {
            let mut rounded = stock[i].round();
            if rounded == -0.0 {
                rounded = 0.0;
            }
            if (stock[i] - rounded).abs() < threshold {
                stock[i] = rounded;
            }
            
        }
        
    }  
    
    total_dur += start.elapsed();
    
    if !is_satisfiable {
        let _ = write!(output_file, "The linear system is not satisfaible!\n\n")?;
        let _ = write!(output_file, "=============================================\n\n\n")?;
    } else {
        write!(output_file, "The linear system is satisfaible!\n")?;
        let _ = write!(output_file, "Results: {{");
        for i in 0..non_basics_len {
            if i != 0 {
                let _ = write!(output_file, ", ")?;
            }
            let (var, position) = &memory[i];
            let _ = write!(output_file, "\"{}\": {}",var ,stock[*position])?;
        }
        let _ = write!(output_file, " }}\n\n")?;
        let _ = write!(output_file, "=============================================\n\n")?;
    }
    
    
    Ok(total_dur)
}   

fn suitable_var(
    pos: usize, 
    matrix: &[f64],            
    len: usize, 
    violation_type: VariableState, 
    ranges: &[(VariableState, f64)],
    stock: &[f64], 
    order: Vec<String>,
    vars: &mut HashMap<String, usize>
) -> usize {

    let basics_len = ranges.len() - len;
    const EPSILON: f64 = 1e-9; 
    let lenght = ranges.len();
    
    for i in 0..lenght {
        let var = order[i].clone();
        if vars.contains_key(&var) {
            let Some(j) = vars.get(&var) else {todo!()};
            let matrix_index = len * pos + j; 
            let index = basics_len + j;
            
            let coeff = matrix[matrix_index];
            if coeff.abs() < EPSILON {continue}; 
            
            let (nb_bound_type, bound_value) = &ranges[index]; 
            
            let needs_to_increase = match violation_type {
                VariableState::AtLowerBound => coeff > 0.0,
                VariableState::AtUpperBound => coeff < 0.0,
                _ => false,
            };
            
            
            if check_bounds(nb_bound_type.clone(), *bound_value, needs_to_increase, stock[index]) {
                return *j;
            }
        }
    }
    
    usize::MAX
}

fn check_bounds(
    nb_bound_type: VariableState, 
    value: f64, 
    needs_to_increase: bool,
    current_stock: f64
) -> bool {
    const EPSILON: f64 = 1e-9; 
    
    match nb_bound_type {
        VariableState::AtLowerBound => {
            if needs_to_increase {
                true 
            } else {
                current_stock > value + EPSILON 
            }
        }
        VariableState::AtUpperBound => {
            if needs_to_increase {
                current_stock < value - EPSILON 
            } else {
                true 
            }
        }
        VariableState::EqualBound => {
            false 
        }
        VariableState::Free => {
            true
        }
    }
}

fn check_range(
ranges: &[(VariableState, f64)], 
stock: &[f64],
order: Vec<String>,
basic: HashMap<String, usize>
) -> (usize, bool, f64, VariableState) {
    
    const EPSILON: f64 = 1e-9; // Ignores minor rounding noise
    
    for i in 0..stock.len() {
        let var = order[i].clone();
        if basic.contains_key(&var){
            let Some(j) = basic.get(&var) else {todo!()};
            
            let current_stock = stock[*j];
            let (state, value) = &ranges[*j];
            
            let _violation = match state {
                VariableState::AtLowerBound => {
                    if current_stock <= value - EPSILON {return (*j, false, *value, VariableState::AtLowerBound)};
                }
                VariableState::AtUpperBound => {
                   if current_stock >= value + EPSILON {return (*j, false, *value, VariableState::AtUpperBound)};
                }
                VariableState::EqualBound => {
                    let mut condition = current_stock <= value - EPSILON;
                    if condition {return (*j, false, *value, VariableState::AtLowerBound)};
                    condition = current_stock >= value + EPSILON;
                    if condition {return (*j, false, *value, VariableState::AtUpperBound)};
                }
                _ => {}
            };
        }
    }
    (0, true, 0.0, VariableState::Free)
}

fn params_discover(
tc: &mut TreeCursor, 
src: &str,
params: &mut HashMap<String, i32>,
) -> () {

    let mut name = tc.node().grammar_name();
    let mut id_string = String::new();
    let mut num = 0;  

    //goes to formula (see grammar)
    tc.goto_first_child();
    //goes to first term
    tc.goto_first_child();
    
    while name != "parameters" && tc.goto_next_sibling() {
        name = tc.node().grammar_name();
    }
    
    
    while tc.goto_next_sibling() {
        name = tc.node().grammar_name();
        if name.eq(IDENTIFIER) {
            let node = tc.node();
            let start = node.start_byte();
            let end = node.end_byte();
                
            if let Some(slice) = src.get(start..end) {
                id_string = slice.to_string();
            }
                
            while name != "inter" {
                tc.goto_next_sibling();
                name = tc.node().grammar_name();
            }
                
            let node = tc.node();
            let start = node.start_byte();
            let end = node.end_byte();
            let slice = &src.as_bytes()[start..end];
            if let Ok(s) = std::str::from_utf8(slice) {
                num = s.parse::<i32>().unwrap();
            }
                
            params.entry(id_string.clone()).or_insert(num);
        }
    }
}

fn vars_discover(
tc: &mut TreeCursor, 
src: &str, 
vars: &mut HashMap<String, usize>,
params: &mut HashMap<String, i32>,
) -> i32 {

    let mut name = tc.node().grammar_name();
    let mut number_of_ands:i32 = 0;
    let mut sibling_exists = true;

    
    
    if !name.eq(IDENTIFIER) {
    
        if name.eq(AND) {
            number_of_ands+=1;
        }

        if tc.goto_first_child() {
            number_of_ands += vars_discover(tc, &src, vars, params);
        }
            
        while tc.goto_next_sibling() {
            number_of_ands += vars_discover(tc, &src, vars, params);
        }
    }else{
        
                
        while name.eq(IDENTIFIER) && sibling_exists {
  
            let node = tc.node();
            let start = node.start_byte();
            let end = node.end_byte();
            
            if let Some(slice) = src.get(start..end) {
                let id_string = slice.to_string();
                let pos = vars.len();
                
                
                if !params.contains_key(&id_string) {
                    vars.entry(id_string.clone()).or_insert(pos);
                }
            }
            
            sibling_exists = tc.goto_next_sibling();
            name = tc.node().grammar_name();
            
            while !name.eq(IDENTIFIER) && sibling_exists{
                sibling_exists = tc.goto_next_sibling();
                name = tc.node().grammar_name();
            }
        }
    }
    
    tc.goto_parent();
    number_of_ands
}


fn dfs_mod(
tc : &mut TreeCursor, 
src : &str, 
left : &mut f64, 
ranges: &mut Vec<(VariableState,f64)>, 
vars: &mut HashMap<String, usize>, 
matrix: &mut Vec<f64>, 
num_eq: &mut i32,
mut state: VariableState, 
mut minus: bool,
params: HashMap<String, f64>
) {

    let mut name = tc.node().grammar_name();
    
    if name.eq(OP) {
        tc.goto_first_child();
        name = tc.node().grammar_name();
        let _violation = match name {
            "=" => state = VariableState::EqualBound,
            "<=" => state = VariableState::AtUpperBound,
            ">=" => state = VariableState::AtLowerBound,
            _ => {},
        };
        tc.goto_parent();
        *left = -1.0;
    }
    
    if name.eq(AND) {
        *num_eq += 1;
        *left = 1.0;
    }
    
    if name.eq("-") {
        minus = true;
    }
    
    if name.eq("parameters") { 
        tc.goto_parent();
        return (); 
    }
    
    if !name.eq(CONSTANT) && !name.eq(IDENTIFIER) {
    
        if tc.goto_first_child() {
            dfs_mod(tc, &src, left, ranges, vars, matrix, num_eq, state.clone(), minus, params.clone());
            minus=false;
        }
        
        while tc.goto_next_sibling() {
            dfs_mod(tc, &src, left, ranges, vars, matrix, num_eq, state.clone(), minus, params.clone());
            minus = false;
        }
    }else{
        let num_eq = num_eq.clone();
        let left = left.clone();
        let mut num: f64 = 1.0;
        let mut id: String = String::new();
        let mut id_found = false; 
        let mut sibling_exists = true;
        
        if name.eq(CONSTANT) {
            //extract the number in string
            
            let node = tc.node();
            let start = node.start_byte();
            let end = node.end_byte();
            let slice = &src.as_bytes()[start..end];
            if let Ok(s) = std::str::from_utf8(slice) {
                    num = s.parse::<f64>().unwrap();
            }
            
            sibling_exists = tc.goto_next_sibling();
            name = tc.node().grammar_name();
        }
        
        

        
        while name.eq(IDENTIFIER) && sibling_exists {
            
            let node = tc.node();
            let start = node.start_byte();
            let end = node.end_byte();
            
            if let Some(slice) = src.get(start..end) {
                
                let id_temp = slice.to_string();
                if params.contains_key(&id_temp) {
                    let Some(n) = params.get(&id_temp) else {panic!("Params should have this parameter {}", id_temp)};
                    num = num * n;
                } else {
                    if id_found { panic!("You cannot put two variables in a term. Only linear equations are accepted")};
                    id_found = true;
                    id = id_temp;
                }
            }
            
            sibling_exists = tc.goto_next_sibling();
            name = tc.node().grammar_name();
            
            while !name.eq(IDENTIFIER) && sibling_exists{
                sibling_exists = tc.goto_next_sibling();
                name = tc.node().grammar_name();
            }
        }
        
        if minus {
            num *= -1.0;
            //minus = false;
        }

        //insert numbers in matrix
        //keep track of ranges
         
        
       if id == "" {
            if let Some(range) = ranges.get_mut(num_eq as usize) {
                let modifier = (-1.0 * left) * num;
                range.0 = state;
                range.1 += modifier;
            } else {
                panic!("Index {} out of bounds for ranges", num_eq);
                }
        } else {
            let var_id = vars.get(&id).expect("Expected an id for the variable");
            let total_vars:i32 = vars.len().try_into().unwrap();
            let mut matrix_idx: usize = (num_eq * total_vars).try_into().unwrap();
            matrix_idx += var_id;
            matrix[matrix_idx] += left * num;
        }

    }
    
    tc.goto_parent();
        
}



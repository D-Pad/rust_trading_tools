
pub fn capitlize_first_letter(input_string: &String) -> String {
    
    let mut new_string: String = String::new();
    let first_char: String = match input_string 
        .to_uppercase()
        .chars()
        .next() 
    {
        Some(c) => c.to_string(),
        None => return input_string.to_string() 
    };

    new_string.push_str(&first_char);
    new_string.push_str(&input_string[1..]);

    new_string
} 



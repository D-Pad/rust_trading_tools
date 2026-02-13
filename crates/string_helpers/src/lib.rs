
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


pub fn multi_line_to_single_line(
    multi_line_str: &str, 
    width: u16
) -> String {
    
    let mut new_msg = String::new();
    let mut c_count: u16 = 0; 
    let mut word_buffer: String = String::new();

    for c in multi_line_str.chars() {
    
        if c != ' ' && c != '\n' {
            word_buffer.push_str(&c.to_string());
            c_count += 1;
        }
        
        else if c == ' ' && word_buffer.len() > 0 {
            new_msg.push_str(&format!("{} ", word_buffer));
            word_buffer = String::new();
            c_count += 1;
        };
        
        if c_count > width {
            new_msg.push_str("\n");
            c_count = word_buffer.len() as u16;
        }
        
    };

    new_msg 
}


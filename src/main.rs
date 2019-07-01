extern crate rand;
use rand::{thread_rng, Rng};

// Given problem and a list of {-1,1}, we say that the assignment satisfies the problem if their
// dot product is 0.
fn get_witness(problem: Vec<i32>, assignment: Vec<i32>) -> Vec<i32> {
    assert!(problem.len() == assignment.len());

    let mut mx = 0;
    let mut sum = 0;
    let mut witness = vec![sum];
    let obfuscator: i32 = 1 - 2 * thread_rng().gen_range(0, 2);

    for (x, y) in problem.iter().zip(assignment.iter()) {
        assert!(*y == 1 || *y == -1);
        sum += *x * *y * obfuscator;
        witness.push(sum);
        if *x > mx { mx = *x; }
    }

    // make sure it's a satisfying assignment
    assert!(sum == 0);
    let shift = thread_rng().gen_range(0, mx+1);
    witness = witness.iter().map(|x| x + shift).collect();

    return witness;
}

fn main() {
    let problem = vec![4, 11, 8, 1];
    let assignment = vec![1, -1, 1, -1];
    let witness = get_witness(problem, assignment);
    println!("witness: {:?}", witness);
}

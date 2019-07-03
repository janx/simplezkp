extern crate rand;
use rand::{thread_rng, Rng};
use blake2b_simd::{blake2b, Hash};

struct ZkMerkleTree {
    data: Vec<i32>,
    tree: Vec<Hash>
}

impl ZkMerkleTree {
    fn new(d: &Vec<i32>) -> ZkMerkleTree {
        let mut data = d.clone();

        let next_power_of_2 = 1 << (((data.len() as f32).log2().ceil()) as i32);
        data.extend(vec![0; next_power_of_2 - data.len()]);

        let rand_list: Vec<i32> = data.iter().map(|_| thread_rng().gen::<i32>().abs()).collect();
        let mut mixed_data = vec![];
        for (x, y) in data.iter().zip(rand_list.iter()) {
            mixed_data.push(*x);
            mixed_data.push(*y);
        }

        let mut tree: Vec<Hash> = mixed_data.iter()
            .map(|_| blake2b(b"") ).collect();
        tree.extend(mixed_data.iter().map(|x| blake2b(&x.to_string().into_bytes()) ));
        for i in (1..mixed_data.len()).rev() {
            let s = [tree[i*2].as_bytes(), tree[i*2 + 1].as_bytes()].concat();
            tree[i] = blake2b(&s);
        }

        ZkMerkleTree {
            data: mixed_data,
            tree: tree
        }
    }

    fn verify_merkle_path(root: &Hash, data_size: usize, value_id: usize, value: i32, path: &Vec<Hash>) -> bool {
        let mut cur = blake2b(&value.to_string().into_bytes());
        let mut tree_node_id = 2 * (value_id + (1 << ((data_size as f32).log2().ceil() as i32)));
        for sibling in path {
            assert!(tree_node_id > 1);
            if tree_node_id % 2 == 0 {
                cur = blake2b(&[cur.as_bytes(), sibling.as_bytes()].concat());
            } else {
                cur = blake2b(&[sibling.as_bytes(), cur.as_bytes()].concat());
            }
            tree_node_id = tree_node_id / 2;
        }
        assert!(tree_node_id == 1);
        *root == cur
    }

    fn get_root(&self) -> Hash {
        self.tree[1]
    }

    fn get_val_and_path(&self, id: usize) -> (i32, Vec<Hash>) {
        let mut _id = id * 2;
        let val = self.data[_id];
        let mut auth_path: Vec<Hash> = vec![];

        _id = _id + self.data.len();
        while _id > 1 {
            auth_path.push(self.tree[_id ^ 1]);
            _id = _id / 2;
        }

        (val, auth_path)
    }
}

struct Proof {
    roots: Vec<Hash>,
    indices: Vec<u8>,
    values: Vec<i32>,
    auth_paths: Vec<Vec<Hash>>
}

impl Proof {
    fn new() -> Proof {
        Proof {
            roots: vec![],
            indices: vec![],
            values: vec![],
            auth_paths: vec![]
        }
    }

    fn get_seed(problem: &Vec<i32>) -> Vec<String> {
        problem.iter().map(|i| i.to_string()).collect()
    }

    fn generate(problem: &Vec<i32>, assignment: &Vec<i32>, num_queries: usize) -> Proof {
        println!("*** Proof Generation ***");
        let mut proof = Proof::new();
        let mut seed: Vec<String> = Proof::get_seed(problem);

        for i in 0..num_queries {
            let witness = get_witness(problem, assignment);
            println!("[round {}] witness: {:?}", i, witness);

            let tree = ZkMerkleTree::new(&witness);
            println!("[round {}] mt root: {:?} size: {} leafs: {}", i, tree.get_root(), tree.tree.len(), tree.data.len());

            let idx = blake2b(&seed.join("").into_bytes()).as_bytes()[0] % ((problem.len()+1) as u8);
            println!("[round {}] random idx: {}", i, idx);

            let i1 = idx as usize;
            let i2 = ((idx + 1) % (witness.len() as u8)) as usize;
            let (v1, ap1) = tree.get_val_and_path(i1);
            let (v2, ap2) = tree.get_val_and_path(i2);
            println!("[round {}] element[{}]={}, auth_path={:?}, element[{}]={}, auth_path={:?}", i, i1, v1, ap1, i2, v2, ap2);
            proof.roots.push(tree.get_root());
            proof.indices.push(idx);
            proof.values.push(v1);
            proof.values.push(v2);
            proof.auth_paths.push(ap1);
            proof.auth_paths.push(ap2);

            seed.push(proof.roots[proof.roots.len()-1].to_hex().to_string());
            seed.push(idx.to_string());
        }

        proof
    }

    fn verify(problem: &Vec<i32>, proof: &Proof) -> bool {
        println!("*** Proof Verification ***");
        let mut valid = true;
        let mut seed = Proof::get_seed(problem);

        for (i, root) in proof.roots.iter().enumerate() {
            let idx = blake2b(&seed.join("").into_bytes()).as_bytes()[0] % ((problem.len()+1) as u8);
            valid &= idx == proof.indices[i];
            let idx = idx as usize;

            // check witness properties
            if idx < problem.len() {
                let check1 = (proof.values[2*i] - proof.values[2*i+1]).abs() == problem[idx].abs();
                println!("[round {}] check witness properties 1: {}", i, check1);
                valid &= check1;
            } else {
                let check2 = proof.values[2*i] == proof.values[2*i+1];
                println!("[round {}] check witness properties 2: {}", i, check2);
                valid &= check2;
            }

            // check authenticate paths
            let i1 = idx;
            let i2 = (idx + 1) % (problem.len() + 1);
            let check1 = ZkMerkleTree::verify_merkle_path(root, problem.len()+1, i1, proof.values[2*i], &proof.auth_paths[2*i]);
            println!("[round {}] check auth_path for element[{}]: {}", i, i1, check1);
            valid &= check1;
            let check2 = ZkMerkleTree::verify_merkle_path(root, problem.len()+1, i2, proof.values[2*i+1], &proof.auth_paths[2*i+1]);
            println!("[round {}] check auth_path for element[{}]: {}", i, i2, check2);
            valid &= check2;

            seed.push(proof.roots[i].to_hex().to_string());
            seed.push(idx.to_string());
        }

        valid
    }
}

// Given problem and a list of {-1,1}, we say that the assignment satisfies the problem if their
// dot product is 0.
fn get_witness(problem: &Vec<i32>, assignment: &Vec<i32>) -> Vec<i32> {
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
    let problem = vec![1, 2, 3, 6, 6, 6, 12];
    let assignment = vec![1, 1, 1, -1, -1, -1, 1];
    let proof = Proof::generate(&problem, &assignment, 100);
    println!("proof verification: {}", Proof::verify(&problem, &proof));
}

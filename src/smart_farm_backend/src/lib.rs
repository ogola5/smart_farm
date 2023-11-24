#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell,collections::HashMap};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Crop {
    id: u64,
    name: String,
    description: String,
    quantity: u32,
    created_at: u64,
    updated_at: Option<u64>,
}

impl Storable for Crop {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Crop {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Task {
    id: u64,
    name: String,
    description: String,
    completed: bool,
    crop_id: u64,
    created_at: u64,
}

impl Storable for Task {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Task {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Expense {
    id: u64,
    description: String,
    amount: f64,
    timestamp: u64,
    crop_id: u64, 
}

impl Storable for Expense {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Expense {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static CROP_STORAGE: RefCell<StableBTreeMap<u64, Crop, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));

    static TASKS: RefCell<StableBTreeMap<u64, Task, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2)))
    ));

    static EXPENSES: RefCell<StableBTreeMap<u64, Expense, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3)))
    ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct CropPayload {
    name: String,
    description: String,
    quantity: u32,
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct TaskPayload {
    name: String,
    description: String,
    crop_id: u64,
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct ExpensePayload {
    description: String,
    amount: f64,
    crop_id: u64,
}

#[ic_cdk::query]
fn get_all_crops() -> Result<Vec<Crop>, Error> {
    let crops_map: Vec<(u64, Crop)> = CROP_STORAGE.with(|service| service.borrow().iter().collect());
    let crops: Vec<Crop> = crops_map.into_iter().map(|(_, crop)| crop).collect();

    if !crops.is_empty() {
        Ok(crops)
    } else {
        Err(Error::NotFound {
            msg: "No crops found.".to_string(),
        })
    }
}

#[ic_cdk::query]
fn get_crop(id: u64) -> Result<Crop, Error> {
    match _get_crop(&id) {
        Some(crop) => Ok(crop),
        None => Err(Error::NotFound {
            msg: format!("Crop with id={} not found.", id),
        }),
    }
}

fn _get_crop(id: &u64) -> Option<Crop> {
    CROP_STORAGE.with(|s| s.borrow().get(id))
}

#[ic_cdk::update]
fn create_crop(payload: CropPayload) -> Option<Crop> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment id counter");

    let crop = Crop {
        id,
        name: payload.name,
        description: payload.description,
        quantity: payload.quantity,
        created_at: time(),
        updated_at: None,
    };
    do_insert(&crop);
    Some(crop)
}

fn do_insert(crop: &Crop) {
    CROP_STORAGE.with(|service| service.borrow_mut().insert(crop.id, crop.clone()));
}

#[ic_cdk::update]
fn update_crop(id: u64, payload: CropPayload) -> Result<Crop, Error> {
    let crop_option: Option<Crop> = CROP_STORAGE.with(|service| service.borrow().get(&id));

    match crop_option {
        Some(mut crop) => {
            crop.name = payload.name;
            crop.description = payload.description;
            crop.quantity = payload.quantity;
            crop.updated_at = Some(time());
            do_insert(&crop);
            Ok(crop)
        }
        None => Err(Error::NotFound {
            msg: format!("Crop with id={} not found.", id),
        }),
    }
}

#[ic_cdk::query]
fn generate_crop_report(id: u64) -> Result<String, Error> {
    match _get_crop(&id) {
        Some(crop) => {
            let report = format!(
                "Crop ID: {}\nName: {}\nDescription: {}\nQuantity: {}\nCreated At: {}\nUpdated At: {:?}",
                crop.id, crop.name, crop.description, crop.quantity, crop.created_at, crop.updated_at
            );
            Ok(report)
        }
        None => Err(Error::NotFound {
            msg: format!("Crop with id={} not found.", id),
        }),
    }
}

#[ic_cdk::query]
fn get_all_tasks() -> Result<Vec<Task>, Error> {
    let tasks_map: Vec<(u64, Task)> = TASKS.with(|service| service.borrow().iter().collect());
    let tasks: Vec<Task> = tasks_map.into_iter().map(|(_, task)| task).collect();

    if !tasks.is_empty() {
        Ok(tasks)
    } else {
        Err(Error::NotFound {
            msg: "No tasks found.".to_string(),
        })
    }
}

#[ic_cdk::query]
fn get_task(id: u64) -> Result<Task, Error> {
    match _get_task(&id) {
        Some(task) => Ok(task),
        None => Err(Error::NotFound {
            msg: format!("Task with id={} not found.", id),
        }),
    }
}

fn _get_task(id: &u64) -> Option<Task> {
    TASKS.with(|s| s.borrow().get(id))
}

#[ic_cdk::update]
fn create_task(payload: TaskPayload) -> Option<Task> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment id counter");

    let task = Task {
        id,
        name: payload.name,
        description: payload.description,
        completed: false,
        crop_id: payload.crop_id,
        created_at: time(),
    };
    do_insert_task(&task);
    Some(task)
}

fn do_insert_task(task: &Task) {
    TASKS.with(|service| service.borrow_mut().insert(task.id, task.clone()));
}

#[ic_cdk::update]
fn update_task(id: u64, payload: TaskPayload) -> Result<Task, Error> {
    let task_option: Option<Task> = TASKS.with(|service| service.borrow().get(&id));

    match task_option {
        Some(mut task) => {
            task.name = payload.name;
            task.description = payload.description;
            task.crop_id = payload.crop_id;
            do_insert_task(&task);
            Ok(task)
        }
        None => Err(Error::NotFound {
            msg: format!("Task with id={} not found.", id),
        }),
    }
}

#[ic_cdk::update]
fn complete_task(id: u64) -> Result<Task, Error> {
    match TASKS.with(|service| {
        if let Some(mut task) = service.borrow_mut().remove(&id) {
            task.completed = true;
            service.borrow_mut().insert(id, task.clone());
            Ok(task)
        } else {
            Err(Error::NotFound {
                msg: format!("Task with id={} not found.", id),
            })
        }
    }) {
        Ok(task) => Ok(task),
        Err(err) => Err(err),
    }
}

#[ic_cdk::update]
fn delete_task(id: u64) -> Result<Task, Error> {
    match TASKS.with(|service| service.borrow_mut().remove(&id)) {
        Some(task) => Ok(task),
        None => Err(Error::NotFound {
            msg: format!("Task with id={} not found.", id),
        }),
    }
}

#[ic_cdk::query]
fn get_all_expenses() -> Result<Vec<Expense>, Error> {
    let expenses_map: Vec<(u64, Expense)> = EXPENSES.with(|service| service.borrow().iter().collect());
    let expenses: Vec<Expense> = expenses_map.into_iter().map(|(_, expense)| expense).collect();

    if !expenses.is_empty() {
        Ok(expenses)
    } else {
        Err(Error::NotFound {
            msg: "No expenses found.".to_string(),
        })
    }
}

#[ic_cdk::query]
fn get_expense(id: u64) -> Result<Expense, Error> {
    match _get_expense(&id) {
        Some(expense) => Ok(expense),
        None => Err(Error::NotFound {
            msg: format!("Expense with id={} not found.", id),
        }),
    }
}

fn _get_expense(id: &u64) -> Option<Expense> {
    EXPENSES.with(|s| s.borrow().get(id))
}

#[ic_cdk::update]
fn create_expense(payload: ExpensePayload) -> Option<Expense> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment id counter");

    let expense = Expense {
        id,
        description: payload.description,
        amount: payload.amount,
        timestamp: time(),
        crop_id: payload.crop_id,  // Use the crop_id from the payload
    };
    do_insert_expense(&expense);
    Some(expense)
}


fn do_insert_expense(expense: &Expense) {
    EXPENSES.with(|service| service.borrow_mut().insert(expense.id, expense.clone()));
}

#[ic_cdk::update]
fn update_expense(id: u64, payload: ExpensePayload) -> Result<Expense, Error> {
    let expense_option: Option<Expense> = EXPENSES.with(|service| service.borrow().get(&id));

    match expense_option {
        Some(mut expense) => {
            expense.description = payload.description;
            expense.amount = payload.amount;
            do_insert_expense(&expense);
            Ok(expense)
        }
        None => Err(Error::NotFound {
            msg: format!("Expense with id={} not found.", id),
        }),
    }
}

#[ic_cdk::update]
fn delete_expense(id: u64) -> Result<Expense, Error> {
    match EXPENSES.with(|service| service.borrow_mut().remove(&id)) {
        Some(expense) => Ok(expense),
        None => Err(Error::NotFound {
            msg: format!("Expense with id={} not found.", id),
        }),
    }
}

#[ic_cdk::query]
fn calculate_budget() -> Result<f64, Error> {
    let all_expenses: Vec<Expense> = get_all_expenses().unwrap_or_default();
    let total_expenses: f64 = all_expenses.iter().map(|expense| expense.amount).sum();
    let all_crops: Vec<Crop> = get_all_crops().unwrap_or_default();
    let total_crop_value: f64 = all_crops.iter().map(|crop| crop.quantity as f64).sum();

    if total_expenses > total_crop_value {
        Ok(total_expenses - total_crop_value)
    } else {
        Ok(total_crop_value - total_expenses)
    }
}
#[ic_cdk::query]
fn crop_rotation_recommendations(current_crop: String) -> Result<Vec<String>, Error> {
    let crop_data = load_crop_rotation_data(); // Load your crop rotation data
    if let Some(recommendations) = crop_data.get(&current_crop) {
        Ok(recommendations.clone())
    } else {
        Err(Error::NotFound {
            msg: "No recommendations available for the input crop.".to_string(),
        })
    }
}

fn load_crop_rotation_data() -> HashMap<String, Vec<String>> {
    // Load your crop rotation data here. You can define crop rotation recommendations.
    let mut data = HashMap::new();
    data.insert("wheat".to_string(), vec!["soybean".to_string(), "corn".to_string()]);
    data.insert("corn".to_string(), vec!["wheat".to_string(), "soybean".to_string()]);
    data.insert("soybean".to_string(), vec!["corn".to_string(), "wheat".to_string()]);
    data
}

#[ic_cdk::query]
fn search_crops(query: String, min_quantity: Option<u32>, date_range: Option<(u64, u64)>) -> Result<Vec<Crop>, Error> {
    let lower_case_query = query.to_lowercase();
    let filtered_crops = CROP_STORAGE.with(|storage| {
        storage.borrow()
            .iter()
            .filter(|(_, crop)| {
                crop.name.to_lowercase().contains(&lower_case_query) &&
                min_quantity.map_or(true, |min| crop.quantity >= min) &&
                date_range.map_or(true, |(start, end)| crop.created_at >= start && crop.created_at <= end)
            })
            .map(|(_, crop)| crop.clone())
            .collect::<Vec<Crop>>()
    });

    if filtered_crops.is_empty() {
        Err(Error::NotFound {
            msg: "No matching crops found.".to_string(),
        })
    } else {
        Ok(filtered_crops)
    }
}


#[ic_cdk::query]
fn predict_crop_yield(crop_id: u64) -> Result<u64, Error> {
    match _get_crop(&crop_id) {
        Some(crop) => {
            // Convert the quantity to u64 before multiplying
            let quantity_u64: u64 = crop.quantity.into();
            let predicted_yield = quantity_u64 * 2; // Example: double the current quantity
            Ok(predicted_yield)
        },
        None => Err(Error::NotFound {
            msg: format!("Crop with id={} not found.", crop_id),
        }),
    }
}


#[ic_cdk::update]
fn auto_assign_tasks() -> Result<Vec<Task>, Error> {
    let mut assigned_tasks = Vec::new();
    CROP_STORAGE.with(|storage| {
        for (_, crop) in storage.borrow().iter() {
            if crop.name == "wheat" {
                // Generate new ID for the task
                let id = ID_COUNTER.with(|counter| {
                    let current_value = *counter.borrow().get();
                    // Handling the result of set operation
                    match counter.borrow_mut().set(current_value + 1) {
                        Ok(_) => current_value + 1,
                        Err(e) => {
                            // Handle the error, e.g., log it or return a custom error
                            eprintln!("Failed to set new ID counter value: {:?}", e);
                            return current_value; // Or handle it another way
                        }
                    }
                });

                let task = Task {
                    id,
                    name: "Watering".to_string(),
                    description: "Water the wheat crop".to_string(),
                    completed: false,
                    crop_id: crop.id,
                    created_at: time(),
                };
                do_insert_task(&task);
                assigned_tasks.push(task);
            }
        }
    });
    Ok(assigned_tasks)
}



fn get_timestamp(month: u64, year: u64, day: u64) -> u64 {
    // Placeholder implementation - replace with actual logic
    year * 10000 + month * 100 + day
}

#[ic_cdk::query]
fn monthly_expense_report(month: u64, year: u64) -> Result<f64, Error> {
    let start_of_month = get_timestamp(month, year, 1);
    let end_of_month = get_timestamp(if month == 12 { 1 } else { month + 1 }, if month == 12 { year + 1 } else { year }, 1);

    let total_expense = EXPENSES.with(|storage| {
        storage.borrow()
               .iter()
               .filter(|(_, expense)| expense.timestamp >= start_of_month && expense.timestamp < end_of_month)
               .map(|(_, expense)| expense.amount)
               .sum()
    });
    Ok(total_expense)
}
#[ic_cdk::query]
fn expenses_per_crop(crop_id: u64) -> Result<f64, Error> {
    let total_expense = EXPENSES.with(|storage| {
        storage.borrow()
               .iter()
               .filter(|(_, expense)| expense.crop_id == crop_id)
               .map(|(_, expense)| expense.amount)
               .sum()
    });
    Ok(total_expense)
}




#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
}

ic_cdk::export_candid!();

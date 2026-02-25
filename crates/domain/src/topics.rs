use std::collections::BTreeMap;

/// Topic metadata rendered in TUI role card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Topic {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
}

/// Static topic catalog grouped by category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicCatalog {
    categories: BTreeMap<&'static str, Vec<Topic>>,
}

impl TopicCatalog {
    /// Returns all category names.
    pub fn categories(&self) -> Vec<String> {
        self.categories.keys().map(|c| (*c).to_string()).collect()
    }

    /// Returns whether a category exists.
    pub fn contains_category(&self, category: &str) -> bool {
        self.categories.contains_key(category)
    }

    /// Returns topics for a category.
    pub fn topics_in_category(&self, category: &str) -> Option<&[Topic]> {
        self.categories.get(category).map(Vec::as_slice)
    }

    /// Finds topic by id.
    pub fn topic_by_id(&self, topic_id: &str) -> Option<&Topic> {
        self.categories
            .values()
            .flat_map(|topics| topics.iter())
            .find(|topic| topic.id == topic_id)
    }
}

/// Default catalog values for role/topic selection.
pub fn default_catalog() -> TopicCatalog {
    let animals = vec![
        Topic {
            id: "animals-red-panda",
            title: "Red Panda",
            description: "A tree-loving mammal with a striped tail and calm personality.",
        },
        Topic {
            id: "animals-rhino",
            title: "Rhino",
            description: "A heavy herbivore known for its protective horn and thick hide.",
        },
        Topic {
            id: "animals-butterfly",
            title: "Butterfly",
            description: "A colorful flying insect often seen around flowers.",
        },
        Topic {
            id: "animals-husky",
            title: "Husky",
            description: "A high-energy dog breed built for cold weather and teamwork.",
        },
    ];

    let countries = vec![
        Topic {
            id: "countries-japan",
            title: "Japan",
            description: "An island nation known for sushi, anime, and advanced trains.",
        },
        Topic {
            id: "countries-brazil",
            title: "Brazil",
            description: "The largest South American country and home of the Amazon.",
        },
        Topic {
            id: "countries-egypt",
            title: "Egypt",
            description: "A North African country famous for the pyramids and Nile river.",
        },
        Topic {
            id: "countries-canada",
            title: "Canada",
            description: "A large northern country known for forests, lakes, and hockey.",
        },
    ];

    let foods = vec![
        Topic {
            id: "foods-pizza",
            title: "Pizza",
            description: "A baked dish with crust, sauce, cheese, and many topping options.",
        },
        Topic {
            id: "foods-ramen",
            title: "Ramen",
            description: "A noodle soup served with broth and toppings like egg or pork.",
        },
        Topic {
            id: "foods-burger",
            title: "Burger",
            description: "A sandwich made with a cooked patty inside a bun.",
        },
        Topic {
            id: "foods-taco",
            title: "Taco",
            description: "A folded tortilla filled with meats, vegetables, and sauces.",
        },
    ];

    let mut categories = BTreeMap::new();
    categories.insert("Animals", animals);
    categories.insert("Countries", countries);
    categories.insert("Foods", foods);
    TopicCatalog { categories }
}

use base::entities::Item;

use super::entities::{
    order::OrderType,
    working_levels::{BasicWLProperties, WLId},
};

/// Checks whether one of the working levels has got crossed and returns such a level.
pub fn get_crossed_level<W>(
    current_tick_price: f32,
    created_working_levels: &[Item<WLId, W>],
) -> Option<&Item<WLId, W>>
where
    W: Into<BasicWLProperties> + Clone,
{
    for level in created_working_levels {
        let level_properties: BasicWLProperties = level.properties.clone().into();

        match level_properties.r#type {
            OrderType::Buy => {
                if current_tick_price < level_properties.price {
                    return Some(level);
                }
            }
            OrderType::Sell => {
                if current_tick_price > level_properties.price {
                    return Some(level);
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    #[test]
    #[allow(non_snake_case)]
    fn get_crossed_level__current_tick_price_is_less_than_buy_level_price__should_return_buy_level()
    {
        let created_working_levels = vec![
            Item {
                id: String::from("2"),
                properties: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: 10.0,
                    time: Utc::now().naive_utc(),
                },
            },
            Item {
                id: String::from("1"),
                properties: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: 10.0,
                    time: Utc::now().naive_utc(),
                },
            },
        ];

        let current_tick_price = 9.0;

        let crossed_level = get_crossed_level(current_tick_price, &created_working_levels);

        assert_eq!(crossed_level.unwrap().id, "1");
    }
    
    #[test]
    #[allow(non_snake_case)]
    fn get_crossed_level__current_tick_price_is_greater_than_sell_level_price__should_return_sell_level()
    {
        let created_working_levels = vec![
            Item {
                id: String::from("1"),
                properties: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: 10.0,
                    time: Utc::now().naive_utc(),
                },
            },
            Item {
                id: String::from("2"),
                properties: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: 10.0,
                    time: Utc::now().naive_utc(),
                },
            },
        ];

        let current_tick_price = 11.0;

        let crossed_level = get_crossed_level(current_tick_price, &created_working_levels);

        assert_eq!(crossed_level.unwrap().id, "2");
    }
    
    #[test]
    #[allow(non_snake_case)]
    fn get_crossed_level__current_tick_price_is_greater_than_buy_level_price_and_less_than_sell_level_price__should_return_none()
    {
        let created_working_levels = vec![
            Item {
                id: String::from("1"),
                properties: BasicWLProperties {
                    r#type: OrderType::Buy,
                    price: 10.0,
                    time: Utc::now().naive_utc(),
                },
            },
            Item {
                id: String::from("2"),
                properties: BasicWLProperties {
                    r#type: OrderType::Sell,
                    price: 12.0,
                    time: Utc::now().naive_utc(),
                },
            },
        ];

        let current_tick_price = 11.0;

        let crossed_level = get_crossed_level(current_tick_price, &created_working_levels);

        assert!(crossed_level.is_none());
    }
}

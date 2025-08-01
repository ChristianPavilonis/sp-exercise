# Simple orders API

This is a demo of an orders api using Axum and SQLite.


## Setup
For convenience I committed the sqlite database with it's basic setup.

To setup from scratch. You'll need [sqlx cli](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md#install)
```bash
# delete and recreate the db
rm db/db.sqlite && touch db/db.sqlite

sqlx migrate run
```

There's a .env file included that points to the `db/db.sqlite` file, this let's me use sqlx's macros for compile time SQL checks.
One trade off of this is that it requires me to use signed integers in rust instead of unsigned because sqlite uses signed. In other databases I could use unsigned when required.
Not a big deal for a demo but good to note.


## Running tests

Run tests with `cargo test`. There's some tests in `orders.rs` but all the tests hitting the http endpoints are in `main.rs`

## Run the api

To run the api run `cargo run` and it will launch on port 3000.

## Endpoints

 - get /orders will get all orders
 - post /orders creates an order
   - amount and status fields are required
 - get /orders/{id} will get a single order by id
 - patch /orders/{id} will update only the status of an order
  - only requires the status field
 - delete /orders/{id}



## Approch

I'm familliar with Axum so I decided to use that, Axum comes with some nice tools out of the box:
- Router, let's us define our routes and HTTP verbs to go with them.
- Extractors, they act like middleware allowing convenient ways to deserialize a json body into a struct and it will provide error messages for us and return the proper http status code when validation fails. It would require a bit of customization to get more robust errors.
- Application state, Axum will manage application state for us and allows me to pass the database pool around to different functions.

I've also used sqlx with SQLite on a few projects
- sqlx has an async api, good for concurrency.
- sqlx will manage our migrations so db schema is easy to reproduce.
- sqlx has macros which allows us to get errors on our queries at compile time instead of runtime.
- It did force me to use signed integers in a few places where unsigned would possibly be a better choice, but this maybe only because of sqlite's limitations.

I made a module for orders to contain all the logic there and implemented CRUD functionality on the Order struct, and wrote tests for those in there.

Then I wrote the http endpoints and used my implementation, I also created a custom Error type using thiserror, any error from my implementation will become a 500 error, and any time I encounter a None when trying to get from the database I return a 404.
The internal server error returns a generic error message, this is good for production, but adding some logging with more context would be ideal.
This is also where other error types such as autherization errors could be put.

I wrote tests in `main.rs` to keep things simple testing all the endpoints, including bad input and 404s. I only wrote one test for 500 errrors, starting the app with an incomplete database, because in theory with this api that's the only dependency that could go wrong. 


### Other considerations

For getting all the orders they should be paginated.
A real application would require authentication.
It would perhaps be better to have OrderRequest struct for the create_order endpoint instead of using the internal Order struct, because any changes to the internal Order would be a breaking change to the REST api.





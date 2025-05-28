def greet(name: str) -> str:
    """Return a greeting message."""
    return f"Hello, {name}!"

class Person:
    def __init__(self, name: str, age: int):
        self.name = name
        self.age = age

    def birthday(self):
        self.age += 1
        print(f"{self.name} is now {self.age} years old!")

if __name__ == "__main__":
    # Create a new person
    person = Person("Alice", 30)
    
    # Print greeting
    message = greet(person.name)
    print(message)
    
    # Celebrate birthday
    person.birthday() 
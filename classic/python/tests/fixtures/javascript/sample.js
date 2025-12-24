// Sample JavaScript Module

import React from 'react';
import { useState, useEffect } from 'react';

/**
 * Main application component
 */
export class App extends React.Component {
    constructor(props) {
        this.state = { count: 0 };
    }

    increment() {
        this.setState({ count: this.state.count + 1 });
    }

    render() {
        return <div>{this.state.count}</div>;
    }
}

export const useCounter = (initial = 0) => {
    const [count, setCount] = useState(initial);
    useEffect(() => {
        console.log(`Count: ${count}`);
    }, [count]);
    return [count, () => setCount(count + 1)];
};

function helperFunction(x, y) {
    const sum = x + y;
    return sum * 2;
}

export default App;

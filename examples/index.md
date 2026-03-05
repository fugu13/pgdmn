# DMN Examples

A library of open source DMN (Decision Model and Notation) examples for use with pgdmn.

## Examples

### Simple Approval

**File:** simple-approval.dmn

A single decision table using the UNIQUE hit policy. Evaluates whether an application should be approved or declined based on the applicant's age, risk category, and affordability.

**Source:** [DMN TCK - 0004-simpletable-U](https://github.com/dmn-tck/tck/tree/master/TestCases/compliance-level-2/0004-simpletable-U)
**License:** CC BY-SA (Creative Commons Attribution-ShareAlike)

| Direction | Name         | Type    |
|-----------|--------------|---------|
| Input     | Age          | number  |
| Input     | RiskCategory | string  |
| Input     | isAffordable | boolean |
| Output    | Approval Status | string ("Approved" or "Declined") |

---

### Multi-Output Approval

**File:** multi-output-approval.dmn

Similar to Simple Approval but demonstrates a decision table with multiple output columns. Returns both an approval status and a rate classification in a single decision.

**Source:** [DMN TCK - 0010-multi-output-U](https://github.com/dmn-tck/tck/tree/master/TestCases/compliance-level-2/0010-multi-output-U)
**License:** CC BY-SA (Creative Commons Attribution-ShareAlike)

| Direction | Name         | Type    |
|-----------|--------------|---------|
| Input     | Age          | number  |
| Input     | RiskCategory | string  |
| Input     | isAffordable | boolean |
| Output    | Approval.Status | string ("Approved" or "Declined") |
| Output    | Approval.Rate   | string ("Best" or "Standard") |

---

### Loan Payment Calculation

**File:** loan-payment.dmn

Calculates a monthly loan payment using a FEEL literal expression with the standard amortization formula. Demonstrates structured input types and arithmetic expressions.

**Source:** [DMN TCK - 0008-LX-arithmetic](https://github.com/dmn-tck/tck/tree/master/TestCases/compliance-level-2/0008-LX-arithmetic)
**License:** CC BY-SA (Creative Commons Attribution-ShareAlike)

| Direction | Name            | Type   |
|-----------|-----------------|--------|
| Input     | loan.principal  | number |
| Input     | loan.rate       | number |
| Input     | loan.termMonths | number |
| Output    | payment         | number |

---

### Monthly Payment with Business Knowledge Model

**File:** monthly-payment-bkm.dmn

Calculates a monthly loan payment plus a fee. Demonstrates the Business Knowledge Model (BKM) pattern: a reusable PMT function is defined once and invoked from the decision. Shows how to separate reusable logic from decision-specific wiring.

**Source:** [DMN TCK - 0009-invocation-arithmetic](https://github.com/dmn-tck/tck/tree/master/TestCases/compliance-level-2/0009-invocation-arithmetic)
**License:** CC BY-SA (Creative Commons Attribution-ShareAlike)

| Direction | Name        | Type   |
|-----------|-------------|--------|
| Input     | Loan.amount | number |
| Input     | Loan.rate   | number |
| Input     | Loan.term   | number |
| Input     | fee         | number |
| Output    | MonthlyPayment | number |

---

### Vacation Days

**File:** vacation-days.dmn

Calculates total vacation days for an employee based on age and years of service. Uses multiple sub-decisions with the COLLECT hit policy (MAX aggregation) to compute extra days from different rule sets, then combines them. Demonstrates decision composition and the COLLECT hit policy.

**Source:** [DMN TCK - 0020-vacation-days](https://github.com/dmn-tck/tck/tree/master/TestCases/compliance-level-3/0020-vacation-days)
**License:** CC BY-SA (Creative Commons Attribution-ShareAlike)

| Direction | Name              | Type   |
|-----------|-------------------|--------|
| Input     | Age               | number |
| Input     | Years of Service  | number |
| Output    | Total Vacation Days | number |

---

### Lending

**File:** lending.dmn

A comprehensive lending decision from the DMN specification. Models the full loan application process including eligibility checks, risk scoring, affordability calculations, bureau calls, and routing. Contains 10+ interconnected decisions and multiple BKMs. This is the canonical complex DMN example.

**Source:** [DMN TCK - 0004-lending](https://github.com/dmn-tck/tck/tree/master/TestCases/compliance-level-3/0004-lending)
**License:** CC BY-SA (Creative Commons Attribution-ShareAlike)

| Direction | Name | Type |
|-----------|------|------|
| Input     | ApplicantData (Age, MaritalStatus, EmploymentStatus, ExistingCustomer, Monthly Income/Expenses/Repayments) | structure |
| Input     | RequestedProduct (ProductType, Amount, Rate, Term) | structure |
| Input     | BureauData (CreditScore, Bankrupt) | structure |
| Output    | Strategy | string ("DECLINE", "BUREAU", "THROUGH") |
| Output    | Routing  | string ("DECLINE", "REFER", "ACCEPT") |
| Output    | (and many intermediate decisions) | various |

---

### Loan Comparison

**File:** loan-comparison.dmn

Compares multiple loan products by computing financial metrics (loan amount, down payment, monthly payment, equity at 36 months) and ranking them by different criteria. Demonstrates relations (embedded data tables), iteration with `for`, sorting with `sort`, and BKMs for financial calculations.

**Source:** [DMN TCK - 0014-loan-comparison](https://github.com/dmn-tck/tck/tree/master/TestCases/compliance-level-3/0014-loan-comparison)
**License:** CC BY-SA (Creative Commons Attribution-ShareAlike)

| Direction | Name         | Type   |
|-----------|--------------|--------|
| Input     | RequestedAmt | number |
| Output    | RankedProducts.metricsTable    | list of metrics |
| Output    | RankedProducts.rankByRate      | list of metrics |
| Output    | RankedProducts.rankByDownPmt   | list of metrics |
| Output    | RankedProducts.rankByMonthlyPmt | list of metrics |
| Output    | RankedProducts.rankByEquityPct | list of metrics |

---

### Dinner Decisions

**File:** dinner-decisions.dmn

A decision requirements graph with two linked decisions: first choose a dish based on season and guest count, then choose beverages based on the dish and whether guests have children. A fun, approachable example of chained decisions with the COLLECT hit policy (multiple beverages can be returned).

Note: this file uses DMN 1.1 namespace and Camunda-specific extensions.

**Source:** [Camunda BPM Examples - dinnerDecisions.dmn](https://github.com/camunda/camunda-bpm-examples/tree/master/dmn-engine/dmn-engine-drg)
**License:** Apache 2.0

| Direction | Name                | Type    |
|-----------|---------------------|---------|
| Input     | Season              | string  |
| Input     | Number of Guests    | integer |
| Input     | Guests with children? | boolean |
| Output    | Dish (intermediate) | string  |
| Output    | Beverages           | string (collected list) |

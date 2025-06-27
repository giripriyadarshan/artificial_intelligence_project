import numpy as np
import joblib
import os
from sklearn.model_selection import train_test_split
from sklearn.ensemble import RandomForestClassifier
from sklearn.metrics import accuracy_score, classification_report

# --- Configuration ---
X_DATA_PATH = 'X_train.npy'
Y_DATA_PATH = 'y_train.npy'
MODEL_SAVE_PATH = 'clash_royale_predictor.joblib'

def main():
    """
    Main function to train, evaluate, and save the match outcome predictor.
    """
    print("🤖 ML Model Training script started.")

    # =========================================================================
    # STEP 1 & 2: Load Pre-processed Data
    # =========================================================================
    print(f"\n🚀 STEP 1 & 2: Loading pre-processed data...")

    # Check if data files exist
    if not os.path.exists(X_DATA_PATH) or not os.path.exists(Y_DATA_PATH):
        print(f"❌ Error: Data files not found ('{X_DATA_PATH}', '{Y_DATA_PATH}').")
        print("Please run the 'run_clustering.py' script first to generate these files.")
        return

    X = np.load(X_DATA_PATH)
    y = np.load(Y_DATA_PATH)
    print(f"✅ Loaded feature matrix X with shape: {X.shape}")
    print(f"✅ Loaded target vector y with shape: {y.shape}")

    # =========================================================================
    # STEP 3: Split Data into Training and Testing Sets
    # =========================================================================
    print("\n🚀 STEP 3: Splitting data into training (80%) and testing (20%) sets...")
    X_train, X_test, y_train, y_test = train_test_split(
        X, y, test_size=0.2, random_state=42, stratify=y
    )
    # Using stratify=y is good practice for classification to ensure train/test sets
    # have a similar proportion of target classes.

    print(f"✅ Data split successfully.")
    print(f"   - X_train shape: {X_train.shape}")
    print(f"   - X_test shape:  {X_test.shape}")

    # =========================================================================
    # STEP 4: Train the Random Forest Model
    # =========================================================================
    print("\n🚀 STEP 4: Training the RandomForestClassifier model...")

    # Instantiate the classifier.
    # - n_estimators: The number of decision trees in the "forest". More trees can
    #   improve performance but increase training time.
    # - random_state: Ensures reproducibility of the model's randomness.
    # - n_jobs=-1: Uses all available CPU cores to parallelize and speed up training.
    model = RandomForestClassifier(
        n_estimators=100,
        random_state=42,
        n_jobs=-1,
        class_weight='balanced' # Helpful for imbalanced datasets
    )

    # Train the model on the training data
    model.fit(X_train, y_train)
    print("✅ Model training complete.")

    # =========================================================================
    # STEP 5: Evaluate Model Performance on the Test Set
    # =========================================================================
    print("\n🚀 STEP 5: Evaluating model performance on the unseen test set...")

    # Use the trained model to make predictions on the held-out test data
    y_pred = model.predict(X_test)

    # Calculate and print the model's accuracy
    accuracy = accuracy_score(y_test, y_pred)
    print(f"\n📈 Model Accuracy: {accuracy:.4f}")

    # Generate and print a detailed classification report.
    # This report provides key metrics for evaluating a classifier's quality:
    # - Precision: Ability of the classifier not to label a sample as positive that is negative.
    # - Recall: Ability of the classifier to find all the positive samples.
    # - F1-score: A weighted average of precision and recall.
    print("\n📊 Classification Report:")
    print(classification_report(y_test, y_pred, target_names=['Player B Wins', 'Player A Wins']))

    # =========================================================================
    # STEP 6: Save the Trained Model for Future Use
    # =========================================================================
    print(f"\n🚀 STEP 6: Saving the trained model to '{MODEL_SAVE_PATH}'...")
    # joblib is efficient for saving scikit-learn models and large numpy arrays.
    joblib.dump(model, MODEL_SAVE_PATH)
    print(f"✅ Model successfully saved.")
    print("\n🏁 Model training and evaluation phase complete!")


if __name__ == "__main__":
    main()